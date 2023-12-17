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
