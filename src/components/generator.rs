use libafl::prelude::{
    Generator, Error,
};
use crate::components::{
    PeacockInput,
    ffi::generator_mutate,
};

pub struct PeacockGenerator;

impl<S> Generator<PeacockInput, S> for PeacockGenerator {
    fn generate(&mut self, _state: &mut S) -> Result<PeacockInput, Error> {
        let mut input = PeacockInput::default();
        generator_mutate(input.sequence_mut());
        Ok(input)
    }
}
