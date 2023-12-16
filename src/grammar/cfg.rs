use std::collections::{HashSet, HashMap};
use ahash::RandomState;
use petgraph::{Graph, visit::Bfs};

use crate::grammar::builder::GrammarBuilder;

/// This type represents a non-terminal in a context-free grammar.
#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct NonTerminal(String);

impl NonTerminal {
    pub(crate) fn new<S: Into<String>>(s: S) -> Self {
        Self(s.into())
    }
    
    /// The id of a non-terminal is its name from the grammar files.
    pub fn id(&self) -> &str {
        &self.0
    }
}

/// This type represents a terminal in a context-free grammar.
#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct Terminal(String);

impl Terminal {
    pub(crate) fn new<S: Into<String>>(s: S) -> Self {
        Self(s.into())
    }
    
    /// The data of the terminal.
    pub fn content(&self) -> &str {
        &self.0
    }
}

/// The right-hand-side of a production rule in a context-free grammar is a sequence
/// of terminals and non-terminals, or a sequence of Symbols.
#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum Symbol {
    Terminal(Terminal),
    NonTerminal(NonTerminal),
}

impl Symbol {
    /// Return whether the Symbol is a terminal
    #[inline]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Symbol::Terminal(_))
    }
    
    /// Return whether the Symbol is a non-terminal
    #[inline]
    pub fn is_non_terminal(&self) -> bool {
        matches!(self, Symbol::NonTerminal(_))
    }
}

/// A ProductionRule states how to expand a non-terminal.  
///   
/// The left-hand-side (lhs) of a rule is the non-terminal to expand.
/// The right-hand-side (rhs) of a rule is the sequence of Symbols that are replacing the lhs.
/// 
/// Please note that if a grammar has multiple ways to expand a non-terminal like so:
/// ```json
/// {
///     "<A-OR-B>": [
///         ["'a'"],
///         ["'b'"],
///     ]
/// }
/// ```
/// then multiple `ProductionRules` will be generated, one for each variant.
#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct ProductionRule {
    lhs: NonTerminal,
    rhs: Vec<Symbol>,
}

impl ProductionRule {
    pub(crate) fn new(lhs: NonTerminal, rhs: Vec<Symbol>) -> Self {
        Self {
            lhs,
            rhs,
        }
    }
    
    /// The left-hand-side of a production rule or the non-terminal that is to be expanded.
    pub fn lhs(&self) -> &NonTerminal {
        &self.lhs
    }
    
    /// The right-hand-side of a production rule or the sequence of Symbols that are replacing the left-hand-side.
    pub fn rhs(&self) -> &[Symbol] {
        &self.rhs
    }
    
    pub(crate) fn fixed_hash(&self) -> u64 {
        RandomState::with_seeds(0, 0, 0, 0).hash_one(self)
    }
}

fn is_mixed(rhs: &[Symbol]) -> bool {
    let mut terms = false;
    let mut non_terms = false;
    
    for symbol in rhs {
        terms |= symbol.is_terminal();
        non_terms |= symbol.is_non_terminal();
    }
    
    terms & non_terms
}

fn is_only_non_terminals(rhs: &[Symbol]) -> bool {
    for symbol in rhs {
        if symbol.is_terminal() {
            return false;
        }
    }
    
    true
}

/// A ContextFreeGrammar is a set of production rules that describe how to construct an input.
/// 
/// Use the [`builder()`](ContextFreeGrammar::builder) method to actually create this struct.
pub struct ContextFreeGrammar {
    rules: Vec<ProductionRule>,
    entrypoint: NonTerminal,
}

impl ContextFreeGrammar {
    /// Build a ContextFreeGrammar.
    pub fn builder() -> GrammarBuilder {
        GrammarBuilder::new()
    }
    
    /// Access the production rules of this grammar.
    pub fn rules(&self) -> &[ProductionRule] {
        &self.rules
    }
    
    /// Access the entrypoint non-terminal of this grammar.
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
    
    pub(crate) fn remove_unit_rules(&mut self) {
        let mut i = 0;
        
        while i < self.rules.len() {
            let rule = &self.rules[i];
            
            if rule.rhs().len() == 1 && rule.rhs()[0].is_non_terminal() {
                let old_rule = self.rules.remove(i);
                let Symbol::NonTerminal(to_expand) = &old_rule.rhs[0] else { unreachable!() };
                let mut new_rules = Vec::new();
                
                for other_rule in &self.rules {
                    if to_expand.id() == other_rule.lhs().id() {
                        new_rules.push(ProductionRule::new(
                            old_rule.lhs().clone(),
                            other_rule.rhs.clone(),
                        ));
                    }
                }
                
                self.rules.append(&mut new_rules);
            } else {
                i += 1;
            }
        }
    }
    
    pub(crate) fn remove_mixed_rules(&mut self) {
        let mut terms = HashMap::new();
        
        for rule in &mut self.rules {            
            if is_mixed(rule.rhs()) {
                for j in 0..rule.rhs().len() {
                    if let Symbol::Terminal(term) = &rule.rhs()[j] {
                        let non_term = terms.entry(term.clone()).or_insert_with(|| NonTerminal(format!("(term:{})", term.content()))).clone();
                        rule.rhs[j] = Symbol::NonTerminal(non_term);
                    }
                }
            }
        }
        
        for (term, nonterm) in terms {
            self.rules.push(ProductionRule::new(
                nonterm,
                vec![Symbol::Terminal(term)],
            ));
        }
    }
    
    pub(crate) fn break_rules(&mut self) {
        let mut nonterm_cursor = 0;
        let mut i = 0;
        
        while i < self.rules.len() {
            let rule = &mut self.rules[i];
            
            if rule.rhs().len() > 2 && is_only_non_terminals(rule.rhs()) {
                let len = rule.rhs().len() - 1;
                let symbols: Vec<Symbol> = rule.rhs.drain(0..len).collect();
                
                let nonterm = NonTerminal(format!("(break_rules:{})", nonterm_cursor));
                nonterm_cursor += 1;
                
                rule.rhs.insert(0, Symbol::NonTerminal(nonterm.clone()));
                
                self.rules.push(ProductionRule::new(
                    nonterm,
                    symbols,
                ));
            }
            
            i += 1;
        }
    }
    
    pub(crate) fn convert_to_gnf(&mut self) {
        let mut i = 0;
        
        while i < self.rules.len() {
            if self.rules[i].rhs()[0].is_non_terminal() {
                let mut new_rules = Vec::new();
                let mut old_rule = self.rules.remove(i);
                let Symbol::NonTerminal(nonterm) = old_rule.rhs.remove(0) else { unreachable!() };
                
                for other_rule in &self.rules {
                    if other_rule.lhs().id() == nonterm.id() {
                        let mut new_symbols = other_rule.rhs.clone();
                        new_symbols.extend_from_slice(old_rule.rhs());
                        new_rules.push(ProductionRule::new(
                            old_rule.lhs().clone(),
                            new_symbols,
                        ));
                    }
                }
                
                self.rules.append(&mut new_rules);
            } else {
                i += 1;
            }
        }
    }
    
    pub(crate) fn set_new_entrypoint(&mut self) {
        let nonterm = NonTerminal("(real_entrypoint)".to_string());
        
        self.rules.push(ProductionRule::new(
            nonterm.clone(),
            vec![Symbol::NonTerminal(self.entrypoint.clone())],
        ));
        
        self.entrypoint = nonterm;
    }
    
    pub(crate) fn count_entrypoint_rules(&self) -> usize {
        let mut count = 0;
        
        for rule in &self.rules {
            if rule.lhs().id() == self.entrypoint.id() {
                count += 1;
            }
        }
        
        count
    }
    
    pub(crate) fn is_in_gnf(&self) -> bool {
        for rule in &self.rules {
            let rhs = rule.rhs();
            
            if rhs[0].is_non_terminal() {
                return false;
            }
            
            if let Some(symbols) = rhs.get(1..) {
                for symbol in symbols {
                    if symbol.is_terminal() {
                        return false;
                    }
                }
            }
        }
        
        true
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
        
        println!("{:#?}", cfg.rules());
    }
    
    #[test]
    fn test_duplicate_rules() {
        let cfg = ContextFreeGrammar::builder()
            .peacock_grammar("test-data/grammars/duplicate_rules.json").unwrap()
            .build()
            .unwrap();
        
        println!("{:#?}", cfg.rules());
    }
    
    #[test]
    fn test_unit_rules() {
        let cfg = ContextFreeGrammar::builder()
            .peacock_grammar("test-data/grammars/unit_rules.json").unwrap()
            .build()
            .unwrap();
        
        println!("{:#?}", cfg.rules());
    }
    
    #[test]
    #[should_panic]
    fn test_recursion() {
        let cfg = ContextFreeGrammar::builder()
            .peacock_grammar("test-data/grammars/recursion.json").unwrap()
            .build()
            .unwrap();
        
        println!("{:#?}", cfg.rules());
    }
    
    #[test]
    #[ignore]
    fn test_mixed_rules() {
        let cfg = ContextFreeGrammar::builder()
            .peacock_grammar("test-data/grammars/mixed_rules.json").unwrap()
            .build()
            .unwrap();
        
        println!("{:#?}", cfg.rules());
    }
}
