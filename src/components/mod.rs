pub(crate) mod ffi;
mod input;
mod mutator;
mod generator;

#[cfg(any(feature = "static-loading", feature = "dynamic-loading"))]
pub use ffi::load_generator;

pub use ffi::generator_seed as seed_generator;

pub use input::PeacockInput;
pub use mutator::PeacockMutator;
pub use generator::PeacockGenerator;
