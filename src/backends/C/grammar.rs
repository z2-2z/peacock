use std::collections::HashMap;

use crate::grammar::{
    ContextFreeGrammar,
    Symbol,
};

#[derive(Copy, Clone, Debug)]
pub struct LLTerminal(usize);

impl LLTerminal {
    pub fn id(&self) -> usize {
        self.0
    }
}

#[derive(Copy, Clone, Debug)]
pub struct LLNonTerminal(usize);

impl LLNonTerminal {
    pub fn id(&self) -> usize {
        self.0
    }
}

#[derive(Clone, Debug)]
pub enum LLSymbol {
    Terminal(LLTerminal),
    NonTerminal(LLNonTerminal),
}

pub struct LowLevelGrammar {
    rules: HashMap<usize, Vec<Vec<LLSymbol>>>,
    terminals: Vec<String>,
    nonterminals: Vec<String>,
    entrypoint: LLNonTerminal,
}

impl LowLevelGrammar {
    pub fn from_high_level_grammar(grammar: &ContextFreeGrammar) -> Self {
        let mut rules = HashMap::new();
        let mut nonterm_map = HashMap::new();
        let mut nonterminals = Vec::new();
        let mut term_map = HashMap::new();
        let mut terminals = Vec::new();

        for rule in grammar.rules() {
            let lhs_id = *nonterm_map.entry(rule.lhs().id()).or_insert_with(|| {
                let ret = nonterminals.len();
                nonterminals.push(rule.lhs().id().to_string());
                ret
            });
            let mut ll_symbols = Vec::new();

            for symbol in rule.rhs() {
                match symbol {
                    Symbol::Terminal(term) => {
                        let id = *term_map.entry(term.content()).or_insert_with(|| {
                            let ret = terminals.len();
                            terminals.push(term.content().to_string());
                            ret
                        });
                        ll_symbols.push(LLSymbol::Terminal(LLTerminal(id)));
                    },
                    Symbol::NonTerminal(nonterm) => {
                        let id = *nonterm_map.entry(nonterm.id()).or_insert_with(|| {
                            let ret = nonterminals.len();
                            nonterminals.push(nonterm.id().to_string());
                            ret
                        });
                        ll_symbols.push(LLSymbol::NonTerminal(LLNonTerminal(id)));
                    },
                }
            }

            rules.entry(lhs_id).or_insert_with(Vec::new).push(ll_symbols);
        }

        Self {
            rules,
            terminals,
            nonterminals,
            entrypoint: LLNonTerminal(*nonterm_map.get(grammar.entrypoint().id()).unwrap()),
        }
    }

    pub fn rules(&self) -> &HashMap<usize, Vec<Vec<LLSymbol>>> {
        &self.rules
    }

    pub fn terminals(&self) -> &[String] {
        &self.terminals
    }

    pub fn nonterminals(&self) -> &[String] {
        &self.nonterminals
    }

    pub fn entrypoint(&self) -> &LLNonTerminal {
        &self.entrypoint
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ll() {
        let cfg = ContextFreeGrammar::builder().peacock_grammar("test-data/grammars/unit_rules.json").unwrap().build().unwrap();
        let ll = LowLevelGrammar::from_high_level_grammar(&cfg);
        println!("{:#?}", ll.rules());
        println!("terminals = {:?}", ll.terminals());
        println!("nonterminals = {:?}", ll.nonterminals());
    }
}
