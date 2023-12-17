//! This module contains LibAFL components that can interact with the generated code of the C backend.
//! 
//! The components can interact with the code in two ways:
//! - __dynamically__: If the code is compiled into a shared object, it can be loaded via dlopen().
//!   If you plan to do this, activate the feature `dynamic-loading` and load the the code with `load_generator("path/to/library.so")`.
//! - __statically__: The code can also be directly compiled into the fuzzer via a build script.
//!   If you plan to do this, activate the feature `static-loading` and call `load_generator()` without an argument.
//!   One caveat of this is that the generated code must be compiled into a static archive that is called `libgenerator.a`.
//!   This name is hardcoded into the components.
//! 
//! Either way, it is mandatory that [`load_generator`] is called before fuzzing starts.
//! 
//! ## Examples
//! For an example of dynamic loading see the binary `peacock-fuzz` in `src/bin/fuzz.rs`.    
//! For an example of static loading see the fuzzer in `test-data/static_loading/src/main.rs`.

/* Check the two mutually exclusive features necessary to use the components */
#[cfg(all(feature = "static-loading", feature = "dynamic-loading"))]
std::compile_error!("The two features 'static-loading' and 'dynamic-loading' are mutually exclusive");

#[cfg(not(any(feature = "static-loading", feature = "dynamic-loading")))]
std::compile_error!("One of the features 'static-loading' or 'dynamic-loading' must be activated");

pub(crate) mod ffi;
mod input;
mod mutator;
mod generator;

pub use ffi::load_generator;

pub use ffi::generator_seed as seed_generator;

pub use input::PeacockInput;
pub use mutator::PeacockMutator;
pub use generator::PeacockGenerator;
