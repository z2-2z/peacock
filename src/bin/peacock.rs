use clap::Parser;
use std::path::{PathBuf, Path};
use std::process::Command;
use std::ops::Deref;
use serde::{Serialize, Deserialize};
use ahash::RandomState;
use std::time::Duration;
use nix::sys::signal::Signal;
use libafl::prelude::{
    Input, Error,
    HitcountsMapObserver, StdMapObserver,
    TimeObserver, MaxMapFeedback, CalibrationStage, feedback_or,
    TimeFeedback, CrashFeedback, StdState, CachedOnDiskCorpus,
    OnDiskCorpus,
    StdMutationalStage, IndexesLenTimeMinimizerScheduler,
    StdWeightedScheduler, powersched::PowerSchedule,
    StdFuzzer, ForkserverExecutor, TimeoutForkserverExecutor,
    Fuzzer, HasTargetBytes, Mutator, MutationResult,
    HasRand, feedback_and, TimeoutFeedback, HasCorpus, Corpus,
    Generator, Launcher, EventConfig, tui::ui::TuiUI, tui::TuiMonitor,
};
use libafl_bolts::prelude::{
    UnixShMemProvider, ShMemProvider, ShMem, AsMutSlice,
    current_nanos, StdRand, tuple_list,
    HasLen, OwnedSlice, Named, Rand, Cores,
};
use peacock_fuzz::{
    grammar::ContextFreeGrammar,
    backends::C::CGenerator,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, value_name = "CORES")]
    cores: String,
    
    #[arg(long, value_name = "GRAMMAR")]
    grammar: String,
    
    #[arg(short)]
    output: String,
    
    #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = true)]
    cmdline: Vec<String>,
}

/* Interface to grammar program */
type GrammarMutationFunc = extern "C" fn(buf: *mut usize, len: usize, capacity: usize) -> usize;
type GrammarSerializationFunc = extern "C" fn(seq: *const usize, seq_len: usize, out: *mut u8, out_len: usize) -> usize;
type GrammarSeedFunc = extern "C" fn(seed: usize);

#[allow(non_upper_case_globals)]
static mut grammar_mutate: Option<GrammarMutationFunc> = None;
#[allow(non_upper_case_globals)]
static mut grammar_serialize: Option<GrammarSerializationFunc> = None;
#[allow(non_upper_case_globals)]
static mut grammar_seed: Option<GrammarSeedFunc> = None;

fn mkdir(dir: &str) {
    match std::fs::create_dir(dir) {
        Ok(()) => {},
        Err(err) => if err.kind() != std::io::ErrorKind::AlreadyExists {
            panic!("Could not create directory {}", dir);
        }
    }
}

fn compile_so(output: &Path, input: &Path) {
    let output = Command::new("cc")
        .args(["-o", &output.to_string_lossy(), "-s", "-fvisibility=hidden", "-DMAKE_VISIBLE", "-O3", "-fPIC", "-shared", &input.to_string_lossy(), "-nostdlib"])
        .output()
        .expect("Could not launch C compiler");
    
    if !output.status.success() {
        panic!("Compiling grammar failed");
    }
}

fn get_function<T: Copy>(lib: &libloading::Library, name: &[u8]) -> T {
    let f: libloading::Symbol<T> = unsafe { lib.get(name) }.expect("Could not find function in generator.so");
    let f = f.deref();
    *f
}

fn load_grammar(grammar_file: &str, out_dir: &str) {
    let generator_so = PathBuf::from(format!("{}/generator.so", out_dir));
    let c_file = PathBuf::from(format!("{}/generator.c", out_dir));
    
    mkdir(out_dir);
    if !generator_so.exists() {
        /* Generate code from grammar */
        let cfg = ContextFreeGrammar::builder()
            .peacock_grammar(grammar_file).unwrap()
            .build().unwrap();
        CGenerator::new(&c_file).generate(cfg);
        
        /* Compile code into generator */
        compile_so(&generator_so, &c_file);
    }
    
    unsafe {
        let lib = libloading::Library::new(&generator_so).expect("Could not load generator.so");
        grammar_mutate = Some(get_function::<GrammarMutationFunc>(&lib, b"mutate_sequence"));
        grammar_serialize = Some(get_function::<GrammarSerializationFunc>(&lib, b"serialize_sequence"));
        grammar_seed = Some(get_function::<GrammarSeedFunc>(&lib, b"seed"));
        std::mem::forget(lib);
    }
}

/* Input type */
static mut SERIALIZATION_BUFFER: [u8; 128 * 1024 * 1024] = [0; 128 * 1024 * 1024];

#[derive(Serialize, Deserialize, Clone, Debug, Hash)]
struct PeacockInput {
    sequence: Vec<usize>,
}

impl Input for PeacockInput {
    fn generate_name(&self, _idx: usize) -> String {
        let hash = RandomState::with_seeds(0, 0, 0, 0).hash_one(self);
        format!("peacock-raw-{:016x}", hash)
    }
}

impl HasLen for PeacockInput {
    fn len(&self) -> usize {
        self.sequence.len()
    }
}

impl HasTargetBytes for PeacockInput {
    fn target_bytes(&self) -> OwnedSlice<u8> {
        let len = unsafe {
            grammar_serialize.unwrap_unchecked()(
                self.sequence.as_ptr(),
                self.sequence.len(),
                SERIALIZATION_BUFFER.as_mut_ptr(),
                SERIALIZATION_BUFFER.len()
            )
        };
        debug_assert!(len < unsafe { SERIALIZATION_BUFFER.len() });
        unsafe {
            OwnedSlice::from_raw_parts(SERIALIZATION_BUFFER.as_ptr(), len)
        }
    }
}

impl Default for PeacockInput {
    fn default() -> Self {
        Self {
            sequence: Vec::with_capacity(4096 * 2),
        }
    }
}

/* Mutator */
struct PeacockMutator;

impl Named for PeacockMutator {
    fn name(&self) -> &str {
        "PeacockMutator"
    }
}

impl<S> Mutator<PeacockInput, S> for PeacockMutator
where
    S: HasRand,
{
    fn mutate(&mut self, state: &mut S, input: &mut PeacockInput, _stage_idx: i32) -> Result<MutationResult, Error> {
        let capacity = input.sequence.capacity();
        let len = state.rand_mut().below(input.sequence.len() as u64 + 1) as usize;
        let buf = input.sequence.as_mut_ptr();
        
        unsafe {
            let new_len = grammar_mutate.unwrap_unchecked()(buf, len, capacity);
            debug_assert!(new_len <= capacity);
            input.sequence.set_len(new_len);
        }
        
        Ok(MutationResult::Mutated)
    }
}

/* Generator */
struct PeacockGenerator;

impl<S> Generator<PeacockInput, S> for PeacockGenerator {
    fn generate(&mut self, _state: &mut S) -> Result<PeacockInput, Error> {
        let mut input = PeacockInput::default();
        let capacity = input.sequence.capacity();
        let buf = input.sequence.as_mut_ptr();
        
        unsafe {
            let new_len = grammar_mutate.unwrap_unchecked()(buf, 0, capacity);
            debug_assert!(new_len <= capacity);
            input.sequence.set_len(new_len);
        }
        
        Ok(input)
    }
}

/* Harness */
fn fuzz(args: Args) -> Result<(), Error> {
    let mut run_client = |state: Option<_>, mut mgr, _core_id| {
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
        
        let map_feedback = MaxMapFeedback::tracking(&edges_observer, true, false);
        let crash_feedback = feedback_and!(
            CrashFeedback::new(),
            map_feedback
        );
        let map_feedback = MaxMapFeedback::tracking(&edges_observer, true, false);
        let timeout_feedback = feedback_and!(
            TimeoutFeedback::new(),
            map_feedback
        );
        let mut objective = feedback_or!(
            crash_feedback,
            timeout_feedback
        );
        
        unsafe {
            grammar_seed.unwrap_unchecked()(seed as usize);
        };
        
        let mut state = if let Some(state) = state {
            state
        } else {
            StdState::new(
                StdRand::with_seed(seed),
                CachedOnDiskCorpus::<PeacockInput>::new(&queue_dir, 128)?,
                OnDiskCorpus::new(&crashes_dir)?,
                &mut feedback,
                &mut objective,
            )?
        };

        let mutator = PeacockMutator {};
        
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
            .shmem_provider(&mut shmem_provider)
            .parse_afl_cmdline(args.cmdline.get(1..).unwrap_or(&[]))
            .coverage_map_size(MAP_SIZE)
            .is_persistent(false)
            .build_dynamic_map(edges_observer, tuple_list!(time_observer))?;
        
        let mut executor = TimeoutForkserverExecutor::with_signal(forkserver, timeout, signal)?;
        
        state.load_initial_inputs(
            &mut fuzzer,
            &mut executor,
            &mut mgr,
            &[
                queue_dir,
                crashes_dir,
            ]
        )?;
        
        if state.corpus().count() == 0 {
            let mut generator = PeacockGenerator {};
            state.generate_initial_inputs(
                &mut fuzzer,
                &mut executor,
                &mut generator,
                &mut mgr,
                4096,
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

fn main() {
    let args = Args::parse();
    load_grammar(&args.grammar, &args.output);
    fuzz(args).expect("Could not launch fuzzer");
}
