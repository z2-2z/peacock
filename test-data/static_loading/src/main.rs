use std::path::Path;
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
use libafl_bolts::prelude::{
    UnixShMemProvider, ShMemProvider, ShMem, AsSliceMut,
    current_nanos, StdRand, tuple_list,
    Cores,
};
use peacock_fuzz::components::{
    load_generator,
    PeacockInput,
    PeacockMutator,
    PeacockGenerator,
    seed_generator,
};

fn main() -> Result<(), Error> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    
    load_generator();
    
    let mut run_client = |state: Option<_>, mut mgr: LlmpRestartingEventManager<_, _, _>, _core_id| {
        let output_dir = Path::new("output");
        let queue_dir = output_dir.join("queue");
        let crashes_dir = output_dir.join("crashes");
        const MAP_SIZE: usize = 2_621_440;
        let seed = current_nanos();
        let powerschedule = PowerSchedule::EXPLORE;
        let timeout = Duration::from_secs(10);
        let signal = str::parse::<Signal>("SIGKILL").unwrap();
        let debug_child = cfg!(debug_assertions);
        
        let mut shmem_provider = UnixShMemProvider::new()?;
        let mut shmem = shmem_provider.new_shmem(MAP_SIZE)?;
        shmem.write_to_env("__AFL_SHM_ID")?;
        let shmem_buf = shmem.as_slice_mut();
        std::env::set_var("AFL_MAP_SIZE", format!("{}", MAP_SIZE));
        
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
            .program(&args[0])
            .debug_child(debug_child)
            .parse_afl_cmdline(args.get(1..).unwrap_or(&[]))
            .coverage_map_size(MAP_SIZE)
            .is_persistent(false)
            .timeout(timeout)
            .kill_signal(signal)
            .build_dynamic_map(edges_observer, tuple_list!(time_observer))?;
        
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
        
        let mut stages = tuple_list!(calibration, mutational);

        fuzzer.fuzz_loop(&mut stages, &mut executor, &mut state, &mut mgr)?;
        Ok(())
    };
    
    let shmem_provider = UnixShMemProvider::new()?;
    
    let monitor = libafl::prelude::SimplePrintingMonitor::new();
    
    let cores = Cores::from_cmdline("0").expect("Invalid core specification");
    
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
