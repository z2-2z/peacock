
#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct NonTerminal(String);

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct Terminal(String);

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

pub struct ContextFreeGrammar {
    rules: Vec<ProductionRule>,
    entrypoint: NonTerminal,
}
