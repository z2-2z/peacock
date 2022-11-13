use std::path::Path;

use crate::{
    error::Error,
    grammar::{cfg::ContextFreeGrammar, merge::GrammarMerger},
};

pub struct Automaton {
    //TODO
}

impl Automaton {
    pub fn from_grammars<P>(paths: &[P]) -> Result<Self, Error>
    where
        P: AsRef<Path>,
    {
        let mut merger = GrammarMerger::new();

        for path in paths {
            merger = merger.merge(path)?;
        }

        let mut cfg = ContextFreeGrammar::from_dict(merger.dict())?;
        cfg.convert_to_gnf();

        todo!();
    }
}
