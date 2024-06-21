use libafl_bolts::prelude::{
    Named, Rand,
};
use libafl::prelude::{
    Mutator, MutationResult, Error, HasRand,
};
use std::borrow::Cow;

use crate::components::{
    PeacockInput,
    ffi::generator_mutate,
};

/// This component implements grammar-based mutations.
pub struct PeacockMutator;

impl PeacockMutator {
    /// Create a new mutator.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {}
    }
}

impl Named for PeacockMutator {
    fn name(&self) -> &Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("PeacockMutator");
        &NAME
    }
}

impl<S> Mutator<PeacockInput, S> for PeacockMutator
where
    S: HasRand,
{
    fn mutate(&mut self, state: &mut S, input: &mut PeacockInput) -> Result<MutationResult, Error> {
        let len = state.rand_mut().below(input.sequence().len());
        input.sequence_mut().truncate(len);
        generator_mutate(input.sequence_mut());
        Ok(MutationResult::Mutated)
    }
}
