use std::path::Path;
use std::collections::HashSet;

use crate::{
    parser::{peacock, gramatron},
    grammar::{ContextFreeGrammar, ProductionRule, Symbol, NonTerminal},
    error::{ParsingError, GrammarError},
};

const DEFAULT_ENTRYPOINT: &str = "ENTRYPOINT";

pub struct GrammarBuilder {
    rules: Vec<ProductionRule>,
    optimize: bool,
    entrypoint: String,
}

impl GrammarBuilder {
    pub(crate) fn new() -> Self {
        Self {
            rules: Vec::new(),
            optimize: true,
            entrypoint: DEFAULT_ENTRYPOINT.to_string(),
        }
    }
    
    fn check_entrypoint(&self) -> bool {
        for rule in &self.rules {
            if rule.lhs().id() == self.entrypoint {
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
    /// Parse a single JSON file in peacock's grammar format.
    /// TODO: explain peacock format
    pub fn peacock_grammar<P: AsRef<Path>>(mut self, path: P) -> Result<Self, ParsingError> {
        let mut new_rules = peacock::parse_json(path.as_ref())?;
        self.rules.append(&mut new_rules);
        Ok(self)
    }
    
    /// Parse a single JSON file in [Gramatron](https://github.com/HexHive/Gramatron)'s grammar format.
    pub fn gramatron_grammar<P: AsRef<Path>>(mut self, path: P) -> Result<Self, ParsingError> {
        let mut new_rules = gramatron::parse_json(path.as_ref())?;
        self.rules.append(&mut new_rules);
        Ok(self)
    }
    
    pub fn optimize(mut self, optimize: bool) -> Self {
        self.optimize = optimize;
        self
    }
    
    pub fn entrypoint<S: Into<String>>(mut self, entrypoint: S) -> Self {
        self.entrypoint = entrypoint.into();
        self
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
            NonTerminal::new(self.entrypoint),
        );
        
        if self.optimize {
            cfg.concatenate_terminals();
            cfg.remove_duplicate_rules();
            cfg.remove_unit_rules();
            cfg.remove_unused_rules();
            
            if !cfg.is_in_gnf() {
                cfg.remove_mixed_rules();
                cfg.break_rules();
                cfg.convert_to_gnf();
                cfg.remove_unused_rules();
            }
        }
        
        if cfg.count_entrypoint_rules() > 1 {
            cfg.set_new_entrypoint();
        }
        
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
    
    #[test]
    fn test_gramatron_grammar() {
        let cfg = ContextFreeGrammar::builder()
            .gramatron_grammar("test-data/grammars/gramatron.json").unwrap()
            .build()
            .unwrap();
        println!("{:#?}", cfg.rules());
    }
}
