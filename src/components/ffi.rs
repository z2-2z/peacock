#[cfg(feature = "dynamic-loading")]
use {
    std::path::Path,
    std::ops::Deref,
};

type GrammarMutationFunc = unsafe extern "C" fn(buf: *mut usize, len: usize, capacity: usize) -> usize;
type GrammarSerializationFunc = unsafe extern "C" fn(seq: *const usize, seq_len: usize, out: *mut u8, out_len: usize) -> usize;
type GrammarSeedFunc = unsafe extern "C" fn(seed: usize);
type GrammarUnparseFunc = unsafe extern "C" fn(seq: *mut usize, seq_capacity: usize, input: *const u8, input_len: usize) -> usize;

#[allow(non_upper_case_globals)]
static mut grammar_mutate: Option<GrammarMutationFunc> = None;
#[allow(non_upper_case_globals)]
static mut grammar_serialize: Option<GrammarSerializationFunc> = None;
#[allow(non_upper_case_globals)]
static mut grammar_seed: Option<GrammarSeedFunc> = None;
#[allow(non_upper_case_globals)]
static mut grammar_unparse: Option<GrammarUnparseFunc> = None;

#[cfg(feature = "static-loading")]
extern "C" {
    fn mutate_sequence(buf: *mut usize, len: usize, capacity: usize) -> usize;
    fn serialize_sequence(seq: *const usize, seq_len: usize, out: *mut u8, out_len: usize) -> usize;
    fn seed_generator(seed: usize);
    fn unparse_sequence(seq: *mut usize, seq_capacity: usize, input: *const u8, input_len: usize) -> usize;
}

#[cfg(feature = "static-loading")]
pub fn load_generator() {
    unsafe {
        grammar_mutate = Some(mutate_sequence);
        grammar_serialize = Some(serialize_sequence);
        grammar_seed = Some(seed_generator);
        grammar_unparse = Some(unparse_sequence);
    }
}

#[cfg(feature = "dynamic-loading")]
fn get_function<T: Copy>(lib: &libloading::Library, name: &[u8]) -> T {
    let f: libloading::Symbol<T> = unsafe { lib.get(name) }.expect("Could not find function in generator.so");
    let f = f.deref();
    *f
}

#[cfg(feature = "dynamic-loading")]
pub fn load_generator<P: AsRef<Path>>(path: P) {
    let path = path.as_ref();
    
    unsafe {
        let lib = libloading::Library::new(path).expect("Could not load generator.so");
        grammar_mutate = Some(get_function::<GrammarMutationFunc>(&lib, b"mutate_sequence"));
        grammar_serialize = Some(get_function::<GrammarSerializationFunc>(&lib, b"serialize_sequence"));
        grammar_seed = Some(get_function::<GrammarSeedFunc>(&lib, b"seed_generator"));
        grammar_unparse = Some(get_function::<GrammarUnparseFunc>(&lib, b"unparse_sequence"));
        std::mem::forget(lib);
    }
}

pub(crate) fn generator_mutate(sequence: &mut Vec<usize>) {
    let len = sequence.len();
    let capacity = sequence.capacity();
    let buf = sequence.as_mut_ptr();
    
    let f = unsafe { grammar_mutate }.expect("load_generator() has not been called before fuzzing");
    
    unsafe { 
        let new_len = f(buf, len, capacity);
        sequence.set_len(new_len);
    }
}

pub(crate) fn generator_serialize(sequence: &[usize], output: &mut [u8]) -> usize {
    let seq = sequence.as_ptr();
    let seq_len = sequence.len();
    let out = output.as_mut_ptr();
    let out_len = output.len();
    
    let f = unsafe { grammar_serialize }.expect("load_generator() has not been called before fuzzing");
    
    unsafe {
        f(seq, seq_len, out, out_len)
    }
}

pub fn generator_seed(seed: usize) {
    let f = unsafe { grammar_seed }.expect("load_generator() has not been called before fuzzing");
    
    unsafe {
        f(seed);
    }
}

pub(crate) fn generator_unparse(sequence: &mut Vec<usize>, input: &[u8]) -> bool {
    let seq = sequence.as_mut_ptr();
    let seq_capacity = sequence.capacity();
    let input_len = input.len();
    let input = input.as_ptr();
    
    let f = unsafe { grammar_unparse }.expect("load_generator() has not been called before fuzzing");
    
    let new_len = unsafe {
        f(seq, seq_capacity, input, input_len)
    };
    
    if new_len == 0 {
        return false;
    }
    
    unsafe {
        sequence.set_len(new_len);
    }
    
    true
}
