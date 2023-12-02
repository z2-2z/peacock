use crate::grammar::builder::GrammarBuilder;

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct NonTerminal(String);

impl NonTerminal {
    pub fn new<S: Into<String>>(s: S) -> Self {
        Self(s.into())
    }
    
    pub fn id(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct Terminal(String);

impl Terminal {
    pub fn new<S: Into<String>>(s: S) -> Self {
        Self(s.into())
    }
    
    pub fn content(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum Symbol {
    Terminal(Terminal),
    NonTerminal(NonTerminal),
}

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct ProductionRule {
    lhs: NonTerminal,
    rhs: Vec<Symbol>,
}

impl ProductionRule {
    pub fn new(lhs: NonTerminal, rhs: Vec<Symbol>) -> Self {
        Self {
            lhs,
            rhs,
        }
    }
    
    pub fn lhs(&self) -> &NonTerminal {
        &self.lhs
    }
    
    pub fn rhs(&self) -> &[Symbol] {
        &self.rhs
    }
}

pub struct ContextFreeGrammar {
    rules: Vec<ProductionRule>,
    entrypoint: NonTerminal,
}

impl ContextFreeGrammar {
    pub fn builder() -> GrammarBuilder {
        GrammarBuilder::new()
    }
    
    pub(crate) fn new(rules: Vec<ProductionRule>, entrypoint: NonTerminal) -> Self {
        Self {
            rules,
            entrypoint,
        }
    }
}
