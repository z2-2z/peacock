use clap::Parser;
use std::path::{PathBuf, Path};
use std::process::Command;
use std::ops::Deref;
use serde::{Serialize, Deserialize};
use ahash::RandomState;
use std::time::Duration;
use nix::sys::signal::Signal;
use libafl::prelude::{
    Input, Error, SimpleMonitor, SimpleEventManager,
    HitcountsMapObserver, StdMapObserver,
    TimeObserver, MaxMapFeedback, CalibrationStage, feedback_or,
    TimeFeedback, CrashFeedback, StdState, CachedOnDiskCorpus,
    OnDiskCorpus,
    StdPowerMutationalStage, IndexesLenTimeMinimizerScheduler,
    StdWeightedScheduler, powersched::PowerSchedule,
    StdFuzzer, ForkserverExecutor, TimeoutForkserverExecutor,
    Fuzzer, HasTargetBytes, Mutator, MutationResult,
    HasRand,
};
use libafl_bolts::prelude::{
    UnixShMemProvider, ShMemProvider, ShMem, AsMutSlice,
    current_nanos, StdRand, tuple_list,
    HasLen, OwnedSlice, Named, Rand,
};
use peacock_fuzz::{
    grammar::ContextFreeGrammar,
    backends::C::CGenerator,
};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, value_name = "GRAMMAR")]
    grammar: String,
    
    #[arg(short)]
    output: String,
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
pub struct PeacockMutator;

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

/* Harness */
fn fuzz(args: Args) -> Result<(), Error> {
    const MAP_SIZE: usize = 2_621_440;
    
    let monitor = SimpleMonitor::new(|s| {
        println!("{s}");
    });
    
    let mut mgr = SimpleEventManager::new(monitor);
    
    let mut shmem_provider = UnixShMemProvider::new().unwrap();
    let mut shmem = shmem_provider.new_shmem(MAP_SIZE).unwrap();
    shmem.write_to_env("__AFL_SHM_ID").unwrap();
    let shmem_buf = shmem.as_mut_slice();
    
    std::env::set_var("AFL_MAP_SIZE", format!("{}", MAP_SIZE));
    
    let edges_observer = unsafe { HitcountsMapObserver::new(StdMapObserver::new("shared_mem", shmem_buf)) };
    
    let time_observer = TimeObserver::new("time");
    
    let map_feedback = MaxMapFeedback::tracking(&edges_observer, true, false);
    
    let calibration = CalibrationStage::new(&map_feedback);
    
    let mut feedback = feedback_or!(
        // New maximization map feedback linked to the edges observer and the feedback state
        map_feedback,
        // Time feedback, this one does not need a feedback state
        TimeFeedback::with_observer(&time_observer)
    );
    
    //TODO
    let mut objective = CrashFeedback::new();
    
    let mut state = StdState::new(
        // RNG
        StdRand::with_seed(current_nanos()),
        // Corpus that will be evolved, we keep it in memory for performance
        CachedOnDiskCorpus::<PeacockInput>::new("TODO", 128).unwrap(),
        // Corpus in which we store solutions (crashes in this example),
        // on disk so the user can get them after stopping the fuzzer
        OnDiskCorpus::new("TODO").unwrap(),
        // States of the feedbacks.
        // The feedbacks can report the data that should persist in the State.
        &mut feedback,
        // Same for objective feedbacks
        &mut objective,
    )
    .unwrap();

    let mutator = PeacockMutator {};
    
    let power = StdPowerMutationalStage::new(mutator);
    
    let scheduler = IndexesLenTimeMinimizerScheduler::new(StdWeightedScheduler::with_schedule(
        &mut state,
        &edges_observer,
        Some(PowerSchedule::EXPLORE),
    ));
    
    let mut fuzzer = StdFuzzer::new(scheduler, feedback, objective);
    
    let forkserver = ForkserverExecutor::builder()
        .program("TODO")
        .debug_child(false)
        .shmem_provider(&mut shmem_provider)
        .parse_afl_cmdline(["TODO"])
        .coverage_map_size(MAP_SIZE)
        .is_persistent(false)
        .build_dynamic_map(edges_observer, tuple_list!(time_observer))
        .unwrap();
    
    let timeout = Duration::from_secs(10);
    let signal = str::parse::<Signal>("SIGKILL").unwrap();
    let mut executor = TimeoutForkserverExecutor::with_signal(forkserver, timeout, signal)
        .expect("Failed to create the executor.");
    
    // load initial inputs
    
    let mut stages = tuple_list!(calibration, power);

    fuzzer.fuzz_loop(&mut stages, &mut executor, &mut state, &mut mgr)?;
    
    Ok(())
}

fn main() {
    let args = Args::parse();
    load_grammar(&args.grammar, &args.output);
    fuzz(args).expect("Could not launch fuzzer");
}
