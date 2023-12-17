use clap::Parser;
use std::path::{PathBuf, Path};
use std::process::Command;
use std::time::Duration;
use nix::sys::signal::Signal;
use libafl::prelude::{
    Error,
    HitcountsMapObserver, StdMapObserver,
    TimeObserver, MaxMapFeedback, CalibrationStage, feedback_or,
    TimeFeedback, CrashFeedback, StdState, CachedOnDiskCorpus,
    OnDiskCorpus,
    StdMutationalStage, IndexesLenTimeMinimizerScheduler,
    StdWeightedScheduler, powersched::PowerSchedule,
    StdFuzzer, ForkserverExecutor, TimeoutForkserverExecutor,
    Fuzzer,
     TimeoutFeedback, HasCorpus, Corpus,
    Launcher, EventConfig, tui::ui::TuiUI, tui::TuiMonitor,
    LlmpRestartingEventManager,
};
use libafl_bolts::prelude::{
    UnixShMemProvider, ShMemProvider, ShMem, AsMutSlice,
    current_nanos, StdRand, tuple_list,
    Cores,
};
use peacock_fuzz::{
    grammar::ContextFreeGrammar,
    backends::C::CGenerator,
    components::{
        load_generator,
        PeacockInput,
        PeacockMutator,
        PeacockGenerator,
        seed_generator,
    },
};

const PRELOAD_ENV: &str = "PEACOCK_PRELOAD";

fn mkdir(dir: &str) {
    match std::fs::create_dir(dir) {
        Ok(()) => {},
        Err(err) => if err.kind() != std::io::ErrorKind::AlreadyExists {
            panic!("Could not create directory {}", dir);
        }
    }
}

/// Return true if a is newer than b
fn newer<P1: AsRef<Path>, P2: AsRef<Path>>(a: P1, b: P2) -> bool {
    let a = std::fs::metadata(a).unwrap().modified().unwrap();
    let b = std::fs::metadata(b).unwrap().modified().unwrap();
    a > b
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
pub enum GrammarFormat {
    Peacock,
    Gramatron,
}

impl std::fmt::Display for GrammarFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GrammarFormat::Peacock => write!(f, "peacock"),
            GrammarFormat::Gramatron => write!(f, "gramatron"),
        }
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, value_name = "CORES")]
    cores: String,
    
    #[arg(long, value_name = "GRAMMAR")]
    grammar: String,
    
    #[arg(short)]
    output: String,
    
    #[arg(long, default_value_t = GrammarFormat::Peacock)]
    format: GrammarFormat,
    
    #[arg(short, long)]
    entrypoint: Option<String>,
    
    #[arg(short, long)]
    corpus: Option<String>,
    
    #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = true)]
    cmdline: Vec<String>,
}

fn compile_so(output: &Path, input: &Path) {
    let output = Command::new("cc")
        .args(["-o", &output.to_string_lossy(), "-flto", "-s", "-fvisibility=hidden", "-DMAKE_VISIBLE", "-O3", "-fPIC", "-shared", &input.to_string_lossy(), "-nostdlib"])
        .output()
        .expect("Could not launch C compiler");
    
    if !output.status.success() {
        panic!("Compiling grammar failed");
    }
}

fn load_grammar(grammar_file: &str, grammar_format: GrammarFormat, out_dir: &str, entrypoint: Option<&String>) {
    let generator_so = PathBuf::from(format!("{}/generator.so", out_dir));
    let c_file = PathBuf::from(format!("{}/generator.c", out_dir));
    
    mkdir(out_dir);
    if !generator_so.exists() || newer(grammar_file, &generator_so) {
        println!("Compiling generator.so ...");
        
        /* Generate code from grammar */
        let mut cfg = ContextFreeGrammar::builder();
        
        match grammar_format {
            GrammarFormat::Peacock => cfg = cfg.peacock_grammar(grammar_file).unwrap(),
            GrammarFormat::Gramatron => cfg = cfg.gramatron_grammar(grammar_file).unwrap(),
        }
        
        if let Some(entrypoint) = entrypoint {
            cfg = cfg.entrypoint(entrypoint);
        }
        
        let cfg = cfg.build().unwrap();
        
        CGenerator::new(&c_file).generate(cfg);
        
        /* Compile code into generator */
        compile_so(&generator_so, &c_file);
    }
    
    load_generator(generator_so);
}

/* Harness */
fn fuzz(args: Args) -> Result<(), Error> {
    let mut run_client = |state: Option<_>, mut mgr: LlmpRestartingEventManager<_, _>, _core_id| {
        let output_dir = Path::new(&args.output);
        let queue_dir = output_dir.join("queue");
        let crashes_dir = output_dir.join("crashes");
        const MAP_SIZE: usize = 2_621_440;
        let seed = current_nanos();
        let powerschedule = PowerSchedule::EXPLORE;
        let timeout = Duration::from_secs(10);
        let signal = str::parse::<Signal>("SIGKILL").unwrap();
        
        #[cfg(debug_assertions)]
        let debug_child = true;
        #[cfg(not(debug_assertions))]
        let debug_child = false;
        
        if let Ok(value) = std::env::var(PRELOAD_ENV) {
            std::env::set_var("LD_PRELOAD", value);
            std::env::remove_var(PRELOAD_ENV);
        }
        
        let mut shmem_provider = UnixShMemProvider::new()?;
        let mut shmem = shmem_provider.new_shmem(MAP_SIZE)?;
        shmem.write_to_env("__AFL_SHM_ID")?;
        let shmem_buf = shmem.as_mut_slice();
        
        std::env::set_var("AFL_MAP_SIZE", format!("{}", MAP_SIZE));
        
        let edges_observer = unsafe { HitcountsMapObserver::new(StdMapObserver::new("shared_mem", shmem_buf)) };
        
        let time_observer = TimeObserver::new("time");
        
        let map_feedback = MaxMapFeedback::tracking(&edges_observer, true, false);
        
        let calibration = CalibrationStage::new(&map_feedback);
        
        let mut feedback = feedback_or!(
            map_feedback,
            TimeFeedback::with_observer(&time_observer)
        );
        
        let mut objective = feedback_or!(
            CrashFeedback::new(),
            TimeoutFeedback::new()
        );
        
        seed_generator(seed as usize);
        
        let mut state = if let Some(state) = state {
            state
        } else {
            StdState::new(
                StdRand::with_seed(seed),
                CachedOnDiskCorpus::<PeacockInput>::new(&queue_dir, 128)?,
                OnDiskCorpus::new(crashes_dir)?,
                &mut feedback,
                &mut objective,
            )?
        };

        let mutator = PeacockMutator::new();
        
        let mutational = StdMutationalStage::with_max_iterations(mutator, 0);
        
        let scheduler = IndexesLenTimeMinimizerScheduler::new(StdWeightedScheduler::with_schedule(
            &mut state,
            &edges_observer,
            Some(powerschedule),
        ));
        
        let mut fuzzer = StdFuzzer::new(scheduler, feedback, objective);
        
        let forkserver = ForkserverExecutor::builder()
            .program(&args.cmdline[0])
            .debug_child(debug_child)
            .parse_afl_cmdline(args.cmdline.get(1..).unwrap_or(&[]))
            .coverage_map_size(MAP_SIZE)
            .is_persistent(false)
            .build_dynamic_map(edges_observer, tuple_list!(time_observer))?;
        
        let mut executor = TimeoutForkserverExecutor::with_signal(forkserver, timeout, signal)?;
        
        if let Some(corpus) = &args.corpus {
            state.load_initial_inputs(
                &mut fuzzer,
                &mut executor,
                &mut mgr,
                &[
                    PathBuf::from(corpus),
                ],
            )?;
        }
        
        state.load_initial_inputs(
            &mut fuzzer,
            &mut executor,
            &mut mgr,
            &[
                queue_dir,
                //crashes_dir,
            ]
        )?;
        
        if state.corpus().count() == 0 {
            let mut generator = PeacockGenerator::new();
            state.generate_initial_inputs_forced(
                &mut fuzzer,
                &mut executor,
                &mut generator,
                &mut mgr,
                16,
            )?;
        }
        
        let mut stages = tuple_list!(calibration, mutational);

        fuzzer.fuzz_loop(&mut stages, &mut executor, &mut state, &mut mgr)?;
        Ok(())
    };
    
    let shmem_provider = UnixShMemProvider::new()?;
    
    let tui = TuiUI::new(
        "peacock".to_string(),
        true
    );
    let monitor = TuiMonitor::new(tui);
    //let monitor = libafl::prelude::SimplePrintingMonitor::new();
    
    let cores = Cores::from_cmdline(&args.cores).expect("Invalid core specification");
    
    match Launcher::builder()
        .shmem_provider(shmem_provider)
        .configuration(EventConfig::AlwaysUnique)
        .monitor(monitor)
        .run_client(&mut run_client)
        .cores(&cores)
        .build()
        .launch()
    {
        Err(Error::ShuttingDown) | Ok(()) => Ok(()),
        e => e,
    }
}

pub fn main() {
    let args = Args::parse();
    load_grammar(&args.grammar, args.format, &args.output, args.entrypoint.as_ref());
    fuzz(args).expect("Could not launch fuzzer");
}
