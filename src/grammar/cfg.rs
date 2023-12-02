use std::collections::{HashSet, HashMap};
use ahash::RandomState;
use petgraph::{Graph, visit::Bfs};

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

impl Symbol {
    #[inline]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Symbol::Terminal(_))
    }
    
    #[inline]
    pub fn is_non_terminal(&self) -> bool {
        matches!(self, Symbol::NonTerminal(_))
    }
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
    
    pub(crate) fn fixed_hash(&self) -> u64 {
        RandomState::with_seeds(0, 0, 0, 0).hash_one(self)
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
    
    pub fn rules(&self) -> &[ProductionRule] {
        &self.rules
    }
    
    pub fn entrypoint(&self) -> &NonTerminal {
        &self.entrypoint
    }
    
    pub(crate) fn new(rules: Vec<ProductionRule>, entrypoint: NonTerminal) -> Self {
        Self {
            rules,
            entrypoint,
        }
    }
}

impl ContextFreeGrammar {
    pub(crate) fn concatenate_terminals(&mut self) {
        for rule in &mut self.rules {
            let mut i = 0;
            
            while i + 1 < rule.rhs.len() {
                if rule.rhs[i].is_terminal() && rule.rhs[i + 1].is_terminal() {
                    let Symbol::Terminal(second) = rule.rhs.remove(i + 1) else { unreachable!() };
                    let Symbol::Terminal(first) = &mut rule.rhs[i] else { unreachable!() };
                    first.0.push_str(second.content());
                } else {
                    i += 1;
                }
            }
        }
    }
    
    pub(crate) fn remove_duplicate_rules(&mut self) {
        let mut hashes = HashSet::with_capacity(self.rules.len());
        let mut i = 0;
        
        while i < self.rules.len() {
            let hash = self.rules[i].fixed_hash();
            
            if !hashes.insert(hash) {
                self.rules.remove(i);
            } else {
                i += 1;
            }
        }
    }
    
    pub(crate) fn remove_unused_rules(&mut self) {
        let mut graph = Graph::<&str, ()>::new();
        let mut nodes = HashMap::new();
        
        /* Construct directed graph of non-terminals */
        for rule in &self.rules {
            let src = rule.lhs().id();
            let src = *nodes.entry(src).or_insert_with(|| graph.add_node(src));
            
            for symbol in rule.rhs() {
                if let Symbol::NonTerminal(nonterm) = symbol {
                    let dst = nonterm.id();
                    let dst = *nodes.entry(dst).or_insert_with(|| graph.add_node(dst));
                    
                    graph.add_edge(src, dst, ());
                }
            }
        }
        
        /* Do a BFS from entrypoint */
        let entrypoint = *nodes.get(self.entrypoint.id()).unwrap();
        let mut bfs = Bfs::new(&graph, entrypoint);
        
        while let Some(idx) = bfs.next(&graph) {
            let id = graph.node_weight(idx).unwrap();
            nodes.remove(id);
        }
        
        /* Now `nodes` contains all the non-terminals that are never used */
        let nodes: HashSet<String> = nodes.into_keys().map(|x| x.to_string()).collect();
        let mut i = 0;
        
        while i < self.rules.len() {
            let lhs = self.rules[i].lhs().id();
            
            if nodes.contains(lhs) {
                self.rules.remove(i);
            } else {
                i += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_unused_rules() {
        let cfg = ContextFreeGrammar::builder()
            .peacock_grammar("test-data/grammars/unused_rules.json").unwrap()
            .build()
            .unwrap();
        
        assert_eq!(cfg.rules().len(), 5);
    }
    
    #[test]
    fn test_duplicate_rules() {
        let cfg = ContextFreeGrammar::builder()
            .peacock_grammar("test-data/grammars/duplicate_rules.json").unwrap()
            .build()
            .unwrap();
        
        assert_eq!(cfg.rules().len(), 2);
        
        println!("{:#?}", cfg.rules());
    }
}
