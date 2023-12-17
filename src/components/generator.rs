use libafl::prelude::{
    Generator, Error,
};
use crate::components::{
    PeacockInput,
    ffi::generator_mutate,
};

/// This component generates new inputs from scratch.
pub struct PeacockGenerator;

impl PeacockGenerator {
    /// Create a new generator.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {}
    }
}

impl<S> Generator<PeacockInput, S> for PeacockGenerator {
    fn generate(&mut self, _state: &mut S) -> Result<PeacockInput, Error> {
        let mut input = PeacockInput::default();
        generator_mutate(input.sequence_mut());
        Ok(input)
    }
}
