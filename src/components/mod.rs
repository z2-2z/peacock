//! This module contains LibAFL components that can interact with the generated code of the C backend.
//!
//! The components can interact with the code in two ways:
//! - __dynamically__ (default): If the code is compiled into a shared object, it can be loaded via dlopen()
//!   in `load_generator("path/to/generator.so")`.
//! - __statically__: The code can also be directly compiled into the fuzzer via a build script.
//!   If you plan to do this, activate the feature `static-loading` and call `load_generator()` without an argument.
//!   One caveat of this is that the generated code must be compiled into a static archive that is called `libgenerator.a`.
//!   This name is hardcoded into this library.
//!
//! Either way, it is mandatory that [`load_generator`] is called before fuzzing starts.
//!
//! ## Examples
//! For an example of dynamic loading see the binary `peacock-fuzz` in `src/bin/fuzz.rs`.    
//! For an example of static loading see the fuzzer in `test-data/static_loading/src/main.rs`.

pub(crate) mod ffi;
mod generator;
mod input;
mod mutator;

pub use ffi::{
    generator_seed as seed_generator,
    load_generator,
};

pub use generator::PeacockGenerator;
pub use input::PeacockInput;
pub use mutator::PeacockMutator;
