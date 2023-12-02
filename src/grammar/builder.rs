use std::path::Path;

use crate::{
    parser::json::parse_json,
    ContextFreeGrammar, ProductionRule,
    error::ParsingError,
};

pub struct GrammarBuilder {
    rules: Vec<ProductionRule>,
}

impl GrammarBuilder {
    /// Parse a single JSON file with peacock's grammar format.
    /// TODO: explain peacock format
    pub fn json<P: AsRef<Path>>(&mut self, path: P) -> Result<&mut Self, ParsingError> {
        let mut new_rules = parse_json(path.as_ref())?;
        self.rules.append(&mut new_rules);
        Ok(self)
    }
    
    pub fn build(self) -> ContextFreeGrammar {
        todo!()
    }
}
