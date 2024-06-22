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
    StdFuzzer, ForkserverExecutor,
    Fuzzer,
     TimeoutFeedback, HasCorpus, Corpus,
    Launcher, EventConfig,
    LlmpRestartingEventManager, CanTrack,
};
#[cfg(not(debug_assertions))]
use libafl::prelude::{tui::ui::TuiUI, tui::TuiMonitor};
use libafl_bolts::prelude::{
    UnixShMemProvider, ShMemProvider, ShMem, AsSliceMut,
    current_nanos, StdRand, tuple_list,
    Cores, CoreId,
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
const CC_ENV: &str = "CC";
const MAP_SIZE_ENV: &str = "PEACOCK_MAP_SIZE";

const DEFAULT_MAP_SIZE: usize = 2_621_440;
const DEFAULT_CC: &str = "cc";

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

fn mkdir(dir: &str) {
    match std::fs::create_dir(dir) {
        Ok(()) => {},
        Err(err) => if err.kind() != std::io::ErrorKind::AlreadyExists {
            panic!("Could not create directory {}", dir);
        }
    }
}

/// Return true if a is newer than b
fn is_newer<P1: AsRef<Path>, P2: AsRef<Path>>(a: P1, b: P2) -> bool {
    let a = std::fs::metadata(a).unwrap().modified().unwrap();
    let b = std::fs::metadata(b).unwrap().modified().unwrap();
    a > b
}

fn compile_source(output: &Path, input: &Path) {
    let cc = if let Ok(var) = std::env::var(CC_ENV) {
        var
    } else {
        DEFAULT_CC.to_string()
    };
    
    let output = Command::new(cc)
        .args(["-o", &output.to_string_lossy(), "-flto", "-s", "-fvisibility=hidden", "-DMAKE_VISIBLE", "-Ofast", "-march=native", "-fomit-frame-pointer", "-fno-stack-protector", "-fPIC", "-shared", &input.to_string_lossy(), "-nostdlib"])
        .output()
        .expect("Could not launch C compiler");
    
    if !output.status.success() {
        panic!("Compiling grammar failed");
    }
}

fn generate_source(args: &Args, c_file: &Path) {
    let mut cfg = ContextFreeGrammar::builder();
        
    match &args.format {
        GrammarFormat::Peacock => cfg = cfg.peacock_grammar(&args.grammar).unwrap(),
        GrammarFormat::Gramatron => cfg = cfg.gramatron_grammar(&args.grammar).unwrap(),
    }
    
    if let Some(entrypoint) = &args.entrypoint {
        cfg = cfg.entrypoint(entrypoint);
    }
    
    let cfg = cfg.build().unwrap();
    
    CGenerator::new().generate(c_file, &cfg);
}

fn load_grammar(args: &Args) {
    let generator_so = PathBuf::from(format!("{}/generator.so", &args.output));
    let c_file = PathBuf::from(format!("{}/generator.c", &args.output));
    
    mkdir(&args.output);
    if !generator_so.exists() || is_newer(&args.grammar, &generator_so) {
        println!("Compiling generator.so ...");
        generate_source(args, &c_file);
        compile_source(&generator_so, &c_file);
    }
    
    load_generator(generator_so);
}

/* Harness */
fn fuzz(args: Args) -> Result<(), Error> {
    let mut map_size = if let Ok(value) = std::env::var(MAP_SIZE_ENV) {
        std::env::remove_var(MAP_SIZE_ENV);
        value.parse().expect("Invalid map size speficiation")
    } else {
        DEFAULT_MAP_SIZE
    };
    
    if map_size % 64 != 0 {
        map_size = ((map_size + 63) >> 6) << 6;
    }
    
    let mut run_client = |state: Option<_>, mut mgr: LlmpRestartingEventManager<_, _, _>, core_id: CoreId| {
        let output_dir = Path::new(&args.output);
        let queue_dir = output_dir.join("queue");
        let crashes_dir = output_dir.join("crashes");
        let seed = current_nanos().rotate_left(core_id.0 as u32);
        let powerschedule = PowerSchedule::EXPLORE;
        let timeout = Duration::from_secs(10);
        let signal = str::parse::<Signal>("SIGKILL").unwrap();
        let debug_child = cfg!(debug_assertions);
        
        if let Ok(value) = std::env::var(PRELOAD_ENV) {
            std::env::set_var("LD_PRELOAD", value);
            std::env::remove_var(PRELOAD_ENV);
        }
        
        let mut shmem_provider = UnixShMemProvider::new()?;
        let mut shmem = shmem_provider.new_shmem(map_size)?;
        shmem.write_to_env("__AFL_SHM_ID")?;
        let shmem_buf = shmem.as_slice_mut();
        std::env::set_var("AFL_MAP_SIZE", format!("{}", map_size));
        
        let edges_observer = unsafe { HitcountsMapObserver::new(StdMapObserver::new("shared_mem", shmem_buf)).track_indices() };
        
        let time_observer = TimeObserver::new("time");
        
        let map_feedback = MaxMapFeedback::new(&edges_observer);
        
        let calibration = CalibrationStage::new(&map_feedback);
        
        let mut feedback = feedback_or!(
            map_feedback,
            TimeFeedback::new(&time_observer)
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
        
        let mutational = StdMutationalStage::with_max_iterations(mutator, 1);
        
        let scheduler = IndexesLenTimeMinimizerScheduler::new(
            &edges_observer,
            StdWeightedScheduler::with_schedule(
                &mut state,
                &edges_observer,
                Some(powerschedule),
            )
        );
        
        let mut fuzzer = StdFuzzer::new(scheduler, feedback, objective);
        
        let mut executor = ForkserverExecutor::builder()
            .program(&args.cmdline[0])
            .debug_child(debug_child)
            .parse_afl_cmdline(args.cmdline.get(1..).unwrap_or(&[]))
            .coverage_map_size(map_size)
            .is_persistent(false)
            .timeout(timeout)
            .kill_signal(signal)
            .build_dynamic_map(edges_observer, tuple_list!(time_observer))?;
        
        if state.corpus().count() == 0 {
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
        }
        
        let mut stages = tuple_list!(calibration, mutational);

        fuzzer.fuzz_loop(&mut stages, &mut executor, &mut state, &mut mgr)?;
        Ok(())
    };
    
    let shmem_provider = UnixShMemProvider::new()?;
    
    #[cfg(not(debug_assertions))]
    let monitor = {
        let tui = TuiUI::new(
            "peacock".to_string(),
            true
        );
        TuiMonitor::new(tui)
    };
    
    #[cfg(debug_assertions)]
    let monitor = libafl::prelude::MultiMonitor::new(|s| println!("{}", s));
    
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
    load_grammar(&args);
    fuzz(args).expect("Could not launch fuzzer");
}
