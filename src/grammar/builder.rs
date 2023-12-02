use std::path::Path;
use std::collections::HashSet;

use crate::{
    parser::json::parse_json,
    ContextFreeGrammar, ProductionRule, Symbol, NonTerminal,
    error::{ParsingError, GrammarError},
};

const ENTRYPOINT: &str = "ENTRYPOINT";

pub struct GrammarBuilder {
    rules: Vec<ProductionRule>,
}

impl GrammarBuilder {
    pub(crate) fn new() -> Self {
        Self {
            rules: Vec::new(),
        }
    }
    
    fn check_entrypoint(&self) -> bool {
        for rule in &self.rules {
            if rule.lhs().id() == ENTRYPOINT {
                return false;
            }
        }
        
        true
    }
    
    fn check_non_terminals(&self) -> Option<String> {
        let mut defined_non_terms = HashSet::new();
        
        for rule in &self.rules {
            defined_non_terms.insert(rule.lhs().id());
        }
        
        for rule in &self.rules {
            for symbol in rule.rhs() {
                if let Symbol::NonTerminal(nonterm) = symbol {
                    if !defined_non_terms.contains(nonterm.id()) {
                        return Some(nonterm.id().to_string());
                    }
                }
            }
        }
        
        None
    }
}

impl GrammarBuilder {
    /// Parse a single JSON file with peacock's grammar format.
    /// TODO: explain peacock format
    pub fn peacock_grammar<P: AsRef<Path>>(mut self, path: P) -> Result<Self, ParsingError> {
        let mut new_rules = parse_json(path.as_ref())?;
        self.rules.append(&mut new_rules);
        Ok(self)
    }
    
    pub fn build(self) -> Result<ContextFreeGrammar, GrammarError> {
        if self.check_entrypoint() {
            return Err(GrammarError::MissingEntrypoint);
        }
        
        if let Some(nonterm) = self.check_non_terminals() {
            return Err(GrammarError::MissingNonTerminal(nonterm));
        }
        
        let mut cfg = ContextFreeGrammar::new(
            self.rules,
            NonTerminal::new(ENTRYPOINT),
        );
        
        cfg.concatenate_terminals();
        cfg.remove_duplicate_rules();
        cfg.remove_unit_rules();
        cfg.remove_unused_rules();
        cfg.remove_mixed_rules();
        
        // break rules with more than two non-terminals
        // convert to GNF via expansion
        // set new entrypoint
        
        Ok(cfg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    #[should_panic]
    fn test_missing_refs() {
        ContextFreeGrammar::builder()
            .peacock_grammar("test-data/grammars/invalid-refs.json").unwrap()
            .build()
            .unwrap();
    }
}
