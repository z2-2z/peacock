use clap::Parser;
use std::path::{PathBuf, Path};
use std::process::Command;
use std::ops::Deref;
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
        .args(["-o", &output.to_string_lossy(), "-s", "-fvisibility=hidden", "-DMAKE_VISIBLE", "-O3", "-fPIC", "-shared", &input.to_string_lossy()])
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
    
    mkdir(out_dir);
    if !generator_so.exists() {
        let base_dir = tempfile::TempDir::with_prefix("peacock").expect("Could not create temporary file");
        let c_file = base_dir.path().join("generator.c");
        
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

fn main() {
    let args = Args::parse();
    let grammar_interface = load_grammar(&args.grammar, &args.output);
    
    assert_eq!(
        (grammar_interface.serialize)(std::ptr::null(), 0, std::ptr::null_mut(), 0),
        0
    );
}
