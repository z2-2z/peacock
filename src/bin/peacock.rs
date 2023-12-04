use clap::Parser;
use std::path::{PathBuf, Path};
use std::process::Command;
use std::ops::Deref;
use serde::{Serialize, Deserialize};
use ahash::RandomState;
use libafl::prelude::{
    Input,
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

#[derive(Clone)]
struct GrammarInterface {
    mutate: GrammarMutationFunc,
    serialize: GrammarSerializationFunc,
    seed: GrammarSeedFunc,
}

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

fn load_grammar(grammar_file: &str, out_dir: &str) -> GrammarInterface {
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
    
    let lib = unsafe { libloading::Library::new(&generator_so).expect("Could not load generator.so") };
    let mutate = get_function::<GrammarMutationFunc>(&lib, b"mutate_sequence");
    let serialize = get_function::<GrammarSerializationFunc>(&lib, b"serialize_sequence");
    let seed = get_function::<GrammarSeedFunc>(&lib, b"seed");
    
    std::mem::forget(lib);
    
    GrammarInterface {
        mutate,
        serialize,
        seed,
    }
}

/* Input type */
#[derive(Serialize, Deserialize, Clone, Debug, Hash, Default)]
struct PeacockInput {
    sequence: Vec<usize>,
}

impl Input for PeacockInput {
    fn generate_name(&self, _idx: usize) -> String {
        let hash = RandomState::with_seeds(0, 0, 0, 0).hash_one(self);
        format!("peacock-raw-{:016x}", hash)
    }
}

/* Executor Wrapper */

fn main() {
    let args = Args::parse();
    let grammar_interface = load_grammar(&args.grammar, &args.output);
    
    assert_eq!(
        (grammar_interface.serialize)(std::ptr::null(), 0, std::ptr::null_mut(), 0),
        0
    );
}
