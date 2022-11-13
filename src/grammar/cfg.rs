use std::fmt::{Display, Formatter, Result as FmtResult};
use std::fs::File;
use std::hash::{BuildHasher, Hash, Hasher};
use std::io::BufReader;
use std::path::Path;

use itertools::Itertools;
use ahash::{AHashMap as HashMap, AHashSet as HashSet, RandomState};
use indexmap::IndexMap;
use json_comments::{CommentSettings, StripComments};
use petgraph::{
    algo::is_cyclic_directed,
    matrix_graph::{MatrixGraph, NodeIndex},
    visit::{Bfs as BreadthFirstSearch, Dfs as DepthFirstSearch},
};
use serde_json as json;

use crate::grammar::error::GrammarError;

/// Name of the non-terminal where generation should start
pub(crate) const ENTRYPOINT: &str = "ENTRYPOINT";


/// A non-terminal. While non-terminals are identified
/// by names in the grammar we use integers for better
/// efficiency.
#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub(crate) struct NonTerminal(usize);

impl NonTerminal {
    /// All non-terminals have a unique ID
    fn id(&self) -> usize {
        self.0
    }
}

/// A terminal that points into the `terminals` array
/// of the context free grammar.
#[derive(Clone, Eq, Hash, PartialEq)]
pub(crate) struct Terminal(usize);

impl Terminal {
    /// Terminals are identified by an index into an array with all terminals.
    fn index(&self) -> usize {
        self.0
    }
}

/// The set of symbols in a context-free grammar is the union of
/// terminals and non-terminals.
#[derive(Clone, Hash, PartialEq)]
pub(crate) enum Symbol {
    NonTerminal(NonTerminal),
    Terminal(Terminal),
}

impl Symbol {
    fn is_non_terminal(&self) -> bool {
        match self {
            Symbol::NonTerminal(_) => true,
            _ => false,
        }
    }

    fn is_terminal(&self) -> bool {
        match self {
            Symbol::Terminal(_) => true,
            _ => false,
        }
    }
}

/// A single production rule of the context-free grammar.
#[derive(Clone, Hash)]
pub(crate) struct ProductionRule {
    lhs: NonTerminal,
    rhs: Vec<Symbol>,
}

impl ProductionRule {
    fn fixed_hash(&self) -> u64 {
        let mut hasher = RandomState::with_seed(0).build_hasher();
        self.hash(&mut hasher);
        hasher.finish()
    }

    fn is_left_recursive(&self) -> bool {
        if self.rhs.len() > 1 {
            match &self.rhs[0] {
                Symbol::NonTerminal(nonterm) => nonterm.id() == self.lhs.id(),
                _ => false,
            }
        } else {
            false
        }
    }
}

/// A grammar where the left-hand side of all production rules
/// consists of a single non-terminal.
pub(crate) struct ContextFreeGrammar {
    terminals: Vec<String>,
    rules: Vec<ProductionRule>,
    entrypoint: NonTerminal,
    nonterminal_cursor: usize,
}

/// A helper for parsing. In the grammar file
/// non-terminals are JSON strings enclosed in '<>' and
/// terminals are strings enclosed in single quotes.
enum SymbolString<'a> {
    Terminal(&'a str),
    NonTerminal(&'a str),
}

impl<'a> SymbolString<'a> {
    fn parse(value: &'a str) -> Option<Self> {
        if value.starts_with("<") && value.ends_with(">") {
            if value.len() > 2 {
                Some(SymbolString::NonTerminal(&value[1..value.len() - 1]))
            } else {
                None
            }
        } else if value.starts_with("'") && value.ends_with("'") {
            if value.len() > 2 {
                Some(SymbolString::Terminal(&value[1..value.len() - 1]))
            } else {
                None
            }
        } else {
            if value.len() > 0 {
                Some(SymbolString::Terminal(value))
            } else {
                None
            }
        }
    }
}

/// Whenever ContextFreeGrammar::next_nonterminal() is not accessible
/// due to the gosh dang borrow checker reside to this macro instead.
macro_rules! next_nonterminal {
    ($self:expr) => {{
        let curr = $self.nonterminal_cursor;
        $self.nonterminal_cursor += 1;
        NonTerminal(curr)
    }};
}

impl ContextFreeGrammar {
    /// Creates a context-free grammar from a grammar specified in a JSON file.
    /// For details on grammar syntax see the documentation in the repository.
    pub(crate) fn from_json(grammar: &json::Value) -> Result<Self, GrammarError> {
        /* Verify that grammar has correct structure */
        let mut non_terminals = IndexMap::<String, usize>::new();
        let mut terminals = IndexMap::<String, usize>::new();

        match &grammar {
            json::Value::Object(map) => {
                for (key, value) in map {
                    let varstring = SymbolString::parse(key);

                    if let Some(var) = varstring {
                        match var {
                            SymbolString::NonTerminal(name) => {
                                let id = non_terminals.len();
                                non_terminals.insert(name.to_string(), id);
                            },
                            SymbolString::Terminal(_) => {
                                return Err(GrammarError::InvalidFormat("Only a single non-terminal is allowed as the left-hand side of a production rule".to_string()));
                            },
                        }
                    } else {
                        return Err(GrammarError::InvalidFormat(format!("Key isn't a non-terminal: '{}'", key)));
                    }

                    match value {
                        json::Value::Array(choices) => {
                            if choices.is_empty() {
                                return Err(GrammarError::InvalidFormat(format!("Right-hand side of key {} is empty", key)));
                            }

                            for choice in choices {
                                match choice {
                                    json::Value::Array(variables) => {
                                        if variables.is_empty() {
                                            return Err(GrammarError::InvalidFormat("Empty sequences are not allowed".to_string()));
                                        }

                                        for variable in variables {
                                            if let Some(variable) = variable.as_str() {
                                                match SymbolString::parse(variable) {
                                                    None => {
                                                        return Err(GrammarError::InvalidFormat(format!("String on the right-hand side of key {} is neither a terminal nor a non-terminal", key)));
                                                    },
                                                    Some(SymbolString::Terminal(term)) => {
                                                        if !terminals.contains_key(term) {
                                                            let id = terminals.len();
                                                            terminals.insert(term.to_string(), id);
                                                        }
                                                    },
                                                    _ => {},
                                                }
                                            } else {
                                                return Err(GrammarError::InvalidFormat(format!("Nested list must contain strings on the right-hand side of key {}", key)));
                                            }
                                        }
                                    },
                                    _ => {
                                        return Err(GrammarError::InvalidFormat(format!("Right-hand side of key {} must be a list of lists", key)));
                                    },
                                }
                            }
                        },
                        _ => {
                            return Err(GrammarError::InvalidFormat(format!("Right-hand side of key {} must be a list of lists", key)));
                        },
                    }
                }
            },
            _ => {
                return Err(GrammarError::InvalidFormat("Grammar must be an object".to_string()));
            },
        }

        if !non_terminals.contains_key(ENTRYPOINT) {
            return Err(GrammarError::InvalidFormat(format!("Grammar does not contain an <{}>", ENTRYPOINT)));
        }

        /* Construct production rules */
        let mut rules = Vec::<ProductionRule>::new();
        let mut nonterminal_cursor = 0;

        for (key, value) in grammar.as_object().unwrap() {
            let nonterm = match SymbolString::parse(key).unwrap() {
                SymbolString::Terminal(_) => unreachable!(),
                SymbolString::NonTerminal(nonterm) => nonterm,
            };
            let lhs = NonTerminal(*non_terminals.get(nonterm).unwrap());

            for value in value.as_array().unwrap() {
                let rhs = match value {
                    json::Value::Array(values) => {
                        let mut variables = Vec::<Symbol>::new();

                        for value in values {
                            let variable = match SymbolString::parse(value.as_str().unwrap()).unwrap() {
                                SymbolString::Terminal(value) => Symbol::Terminal(Terminal(*terminals.get(value).unwrap())),
                                SymbolString::NonTerminal(value) => {
                                    if let Some(id) = non_terminals.get(value) {
                                        Symbol::NonTerminal(NonTerminal(*id))
                                    } else {
                                        return Err(GrammarError::InvalidFormat(format!("Non-terminal '{}' referenced but not defined", value)));
                                    }
                                },
                            };
                            variables.push(variable);
                        }

                        variables
                    },
                    _ => unreachable!(),
                };

                nonterminal_cursor = std::cmp::max(nonterminal_cursor, lhs.id());

                rules.push(ProductionRule {
                    lhs: lhs.clone(),
                    rhs,
                });
            }
        }

        let entrypoint = NonTerminal(*non_terminals.get(ENTRYPOINT).unwrap());
        let terminals = terminals.drain(..).map(|(key, _)| key).collect::<Vec<String>>();

        let mut ret = ContextFreeGrammar {
            terminals,
            rules,
            entrypoint,
            nonterminal_cursor: nonterminal_cursor + 1,
        };
        ret.remove_duplicates();
        ret.check_cycles()?;
        ret.remove_unused_rules();
        Ok(ret)
    }

    /// Check if the current context-free grammar contains infinite loops.
    fn check_cycles(&self) -> Result<(), GrammarError> {
        /* First, identify non-terminals that would terminate a cycle */
        let mut terminators = HashSet::<NonTerminal>::new();

        'next_rule: for rule in &self.rules {
            for var in &rule.rhs {
                if var.is_non_terminal() {
                    continue 'next_rule;
                }
            }

            terminators.insert(rule.lhs.clone());
        }

        /* Build a graph with all non-terminals that don't terminate a cycle */
        let mut nodes = HashMap::<NonTerminal, NodeIndex>::new();
        let mut graph = MatrixGraph::<(), ()>::with_capacity(self.nonterminal_cursor - terminators.len());

        for rule in &self.rules {
            if terminators.contains(&rule.lhs) {
                continue;
            }

            for var in &rule.rhs {
                match var {
                    Symbol::NonTerminal(nonterm) => {
                        if terminators.contains(nonterm) {
                            continue;
                        }

                        let src = if let Some(idx) = nodes.get(&rule.lhs) {
                            idx.clone()
                        } else {
                            let idx = graph.add_node(());
                            nodes.insert(rule.lhs.clone(), idx.clone());
                            idx
                        };

                        let dst = if let Some(idx) = nodes.get(nonterm) {
                            idx.clone()
                        } else {
                            let idx = graph.add_node(());
                            nodes.insert(nonterm.clone(), idx.clone());
                            idx
                        };

                        if !graph.has_edge(src.clone(), dst) {
                            graph.add_edge(src.clone(), dst, ());
                        }
                    },
                    _ => {},
                }
            }
        }

        /* Run a cycle detection algorithm from the graph library */
        if is_cyclic_directed(&graph) {
            Err(GrammarError::ContainsCycles)
        } else {
            Ok(())
        }
    }

    /// Eliminate production rules that cannot be reached from the entrypoint.
    fn remove_unused_rules(&mut self) {
        /* Build a graph from the grammar */
        let mut nodes = HashMap::<NonTerminal, NodeIndex>::new();
        let mut graph = MatrixGraph::<usize, ()>::with_capacity(self.nonterminal_cursor);

        for rule in &self.rules {
            for var in &rule.rhs {
                match var {
                    Symbol::NonTerminal(nonterm) => {
                        let src = if let Some(idx) = nodes.get(&rule.lhs) {
                            idx.clone()
                        } else {
                            let idx = graph.add_node(rule.lhs.id());
                            nodes.insert(rule.lhs.clone(), idx.clone());
                            idx
                        };

                        let dst = if let Some(idx) = nodes.get(nonterm) {
                            idx.clone()
                        } else {
                            let idx = graph.add_node(nonterm.id());
                            nodes.insert(nonterm.clone(), idx.clone());
                            idx
                        };

                        if !graph.has_edge(src.clone(), dst) {
                            graph.add_edge(src.clone(), dst, ());
                        }
                    },
                    _ => {},
                }
            }
        }

        /* Conduct a BFS from the entrypoint */
        let root = *nodes.get(&self.entrypoint).unwrap();
        let mut bfs = BreadthFirstSearch::new(&graph, root);

        while let Some(idx) = bfs.next(&graph) {
            let nonterm = NonTerminal(*graph.node_weight(idx));
            nodes.remove(&nonterm);
        }

        /* Now the nodes-map contains unvisited nodes */
        self.rules.retain(|rule| !nodes.contains_key(&rule.lhs));
    }

    /// Remove duplicate rules
    fn remove_duplicates(&mut self) {
        let mut hashes = HashSet::<u64>::new();
        let mut i = 0;

        while i < self.rules.len() {
            let rule_hash = self.rules[i].fixed_hash();

            if !hashes.insert(rule_hash) {
                self.rules.remove(i);
                continue;
            }

            i += 1;
        }
    }

    /// Get a NonTerminal with a unique ID
    fn next_nonterminal(&mut self) -> NonTerminal {
        let id = self.nonterminal_cursor;
        self.nonterminal_cursor += 1;
        NonTerminal(id)
    }

    /// Create a new non-terminal that produces the current
    /// entrypoint and make it the new entrypoint.
    /// This ensures that the entrypoint of a CFG does not appear
    /// on the right-hand side of any production rule.
    fn new_entrypoint(&mut self) {
        let mut need_new = false;

        for rule in &self.rules {
            for var in &rule.rhs {
                match var {
                    Symbol::NonTerminal(nonterm) => {
                        if nonterm == &self.entrypoint {
                            need_new = true;
                        }
                    },
                    _ => {},
                }
            }
        }

        if need_new {
            let new_entrypoint = self.next_nonterminal();
            let old_entrypoint = self.entrypoint.clone();

            self.rules.push(ProductionRule {
                lhs: new_entrypoint.clone(),
                rhs: vec![Symbol::NonTerminal(old_entrypoint)],
            });

            self.entrypoint = new_entrypoint;
        }

        /* Make sure that the entrypoint always has ID 0 */
        if self.entrypoint.id() != 0 {
            for rule in &mut self.rules {
                if rule.lhs.id() == 0 {
                    rule.lhs = self.entrypoint.clone();
                } else if rule.lhs.id() == self.entrypoint.id() {
                    rule.lhs = NonTerminal(0);
                }
                
                for var in &mut rule.rhs {
                    match var {
                        Symbol::NonTerminal(nonterm) => {
                            if nonterm.id() == 0 {
                                *nonterm = self.entrypoint.clone();
                            } else if nonterm.id() == self.entrypoint.id() {
                                *nonterm = NonTerminal(0);
                            }
                        },
                        _ => {},
                    }
                }
            }
            
            self.entrypoint = NonTerminal(0);
        }
    }

    /// Introduce a new non-terminal for every terminal and replace
    /// its occurences on the right-hand sides of a production rules
    /// with the new non-terminal.
    fn isolate_terminals(&mut self) {
        let mut assoc = HashMap::<Terminal, NonTerminal>::new();

        for rule in &mut self.rules {
            if rule.rhs.len() == 1 {
                continue;
            }

            for var in &mut rule.rhs {
                let term = match var {
                    Symbol::Terminal(term) => term.clone(),
                    _ => {
                        continue;
                    },
                };
                let nonterm = if let Some(nonterm) = assoc.get(&term) {
                    nonterm.clone()
                } else {
                    let nonterm = next_nonterminal!(self);
                    assoc.insert(term, nonterm.clone());
                    nonterm
                };

                *var = Symbol::NonTerminal(nonterm.clone());
            }
        }

        for (term, nonterm) in assoc {
            self.rules.push(ProductionRule {
                lhs: nonterm,
                rhs: vec![Symbol::Terminal(term)],
            });
        }
    }

    /// Make sure that not more than 2 non-terminals appear
    /// on the right-hand side of any production rule.
    fn bin_rhs_cnf(&mut self) {
        let old_len = self.rules.len();

        for i in 0..old_len {
            if self.rules[i].rhs.len() <= 2 {
                continue;
            }

            let rule = self.rules.remove(i);
            let mut rules = vec![rule];

            while rules.last().unwrap().rhs.len() > 2 {
                let rule = rules.last_mut().unwrap();

                let rem_vars = rule.rhs.split_off(1);
                let nonterm = self.next_nonterminal();
                rule.rhs.push(Symbol::NonTerminal(nonterm.clone()));

                rules.push(ProductionRule {
                    lhs: nonterm,
                    rhs: rem_vars,
                });
            }

            self.rules.append(&mut rules);
        }
    }

    /// Eliminate unit rules of the form A -> B for non-terminals
    /// A and B.
    fn eliminate_unit_rules(&mut self) {
        /* Delete unit rules and store them in temporary */
        let mut pairs = Vec::<(NonTerminal, NonTerminal)>::new();
        let mut i = 0;

        while i < self.rules.len() {
            if self.rules[i].rhs.len() == 1 && self.rules[i].rhs[0].is_non_terminal() {
                let rule = self.rules.remove(i);
                let rhs = match &rule.rhs[0] {
                    Symbol::NonTerminal(rhs) => rhs.clone(),
                    _ => unreachable!(),
                };
                pairs.push((rule.lhs, rhs));
                continue;
            }

            i += 1;
        }

        /* Check for transitive unit rules: A -> B -> C yields A -> C */
        i = 0;

        'outer_loop: while i < pairs.len() {
            for j in 0..pairs.len() {
                if pairs[i].1 == pairs[j].0 {
                    let new_pair = (pairs[i].0.clone(), pairs[j].1.clone());

                    if !pairs.contains(&new_pair) {
                        pairs.push(new_pair);
                        i = 0;
                        continue 'outer_loop;
                    }
                }
            }

            i += 1;
        }

        /* Expand right-hand side of unit rules */
        for (src, dst) in pairs {
            for i in 0..self.rules.len() {
                if self.rules[i].lhs == dst {
                    self.rules.push(ProductionRule {
                        lhs: src.clone(),
                        rhs: self.rules[i].rhs.clone(),
                    });
                }
            }
        }
    }

    /// Convert this context-free grammar into Chomsky Normal Form.
    pub(crate) fn convert_to_cnf(&mut self) {
        self.new_entrypoint();
        self.isolate_terminals();
        self.bin_rhs_cnf();
        // The grammar specification disallows epsilon rules so we don't have to remove them here
        self.eliminate_unit_rules();
        self.remove_duplicates();
    }

    /// Check whether this context-free grammar is in Chomsky Normal Form.
    pub(crate) fn is_cnf(&self) -> bool {
        for rule in &self.rules {
            if rule.rhs.len() == 1 {
                if rule.rhs[0].is_non_terminal() {
                    return false;
                }
            } else if rule.rhs.len() == 2 {
                if rule.rhs[0].is_terminal() || rule.rhs[1].is_terminal() {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }

    /// Given a direct left-recursive rule `old_rule`, transform it
    /// into multiple non-left-recursive rules without producing epsilon-rules.
    fn resolve_left_recursion(&mut self, old_rule: ProductionRule) {
        let new_nonterm = self.next_nonterminal();
        let mut new_rule = ProductionRule {
            lhs: new_nonterm.clone(),
            rhs: old_rule.rhs[1..].to_vec(),
        };

        self.rules.push(new_rule.clone());

        new_rule.rhs.push(Symbol::NonTerminal(new_nonterm.clone()));
        self.rules.push(new_rule);

        /* Fix-up non-recursive variants with same lhs */
        for j in 0..self.rules.len() {
            if self.rules[j].lhs.id() == old_rule.lhs.id() {
                if !self.rules[j].is_left_recursive() {
                    let mut new_rule = self.rules[j].clone();
                    new_rule.rhs.push(Symbol::NonTerminal(new_nonterm.clone()));
                    self.rules.push(new_rule);
                }
            }
        }
    }

    /// Break up direct left-recursive rules in the grammar.
    fn remove_direct_left_recursions(&mut self) {
        let mut i = 0;
        let mut old_len = self.rules.len();

        while i < old_len {
            if self.rules[i].is_left_recursive() {
                let rule = self.rules.remove(i);
                old_len -= 1;
                self.resolve_left_recursion(rule);
                continue;
            }

            i += 1;
        }
    }
    
    /// Given a non-terminal A, calculate a matrix that indicates the
    /// common prefix length for every pair of A's production rules.
    fn get_prefix_matrix(&self, nonterm: usize) -> Vec<Vec<usize>> {
        let mut matrix = Vec::<Vec<usize>>::new();
        
        for _ in 0..self.rules.len() {
            matrix.push(vec![0; self.rules.len()]);
        }
        
        for (i, src_rule) in self.rules.iter().enumerate() {
            if src_rule.lhs.id() != nonterm {
                continue;
            }
            
            for (j, dst_rule) in self.rules.iter().enumerate() {
                if j > i && dst_rule.lhs.id() == nonterm {
                    let mut prefix_length = 0;
                    let min_length = std::cmp::min(src_rule.rhs.len(), dst_rule.rhs.len()).saturating_sub(1);
                    
                    while prefix_length < min_length {
                        if src_rule.rhs[prefix_length] != dst_rule.rhs[prefix_length] {
                            break;
                        }
                        
                        prefix_length += 1;
                    }
                    
                    if prefix_length > 1 {
                        matrix[i][j] = prefix_length;
                    }
                }
            }
        }
        
        matrix
    }
    
    /// Find common prefixes in the expansions of non-terminals.
    fn left_factoring(&mut self) {
        let old_cursor = self.nonterminal_cursor;
        for nonterm in 0..old_cursor {
            let mut del_rules = HashSet::<usize>::new();
            let matrix = self.get_prefix_matrix(nonterm);
            
            println!("{}:", nonterm);
            
            for row in &matrix {
                println!("  {:?}", row);
            }
            
            for row in 0..matrix.len() {
                /* Bucket columns by count */
                let mut buckets = HashMap::<usize, Vec<usize>>::new();
                
                for col in 0..matrix[row].len() {
                    let count = matrix[row][col];
                    if count > 0 {
                        if let Some(bucket) = buckets.get_mut(&count) {
                            bucket.push(col);
                        } else {
                            buckets.insert(count, vec![col]);
                        }
                    }
                }
                
                println!("  {}: {:?}", row, buckets);
                
                /* Create new rules with common prefixes */
                for (prefix_length, cols) in buckets {
                    let mut base_rule = ProductionRule {
                        lhs: self.rules[row].lhs.clone(),
                        rhs: self.rules[row].rhs[0..prefix_length].to_vec(),
                    };
                    let new_nonterm = self.next_nonterminal();
                    
                    base_rule.rhs.push(Symbol::NonTerminal(new_nonterm.clone()));
                    self.rules.push(base_rule);
                    
                    let branch_rule = ProductionRule {
                        lhs: new_nonterm.clone(),
                        rhs: self.rules[row].rhs[prefix_length..].to_vec(),
                    };
                    self.rules.push(branch_rule);
                    del_rules.insert(row);
                    
                    for col in cols {
                        let branch_rule = ProductionRule {
                            lhs: new_nonterm.clone(),
                            rhs: self.rules[col].rhs[prefix_length..].to_vec(),
                        };
                        self.rules.push(branch_rule);
                        del_rules.insert(col);
                    }
                }
            }
            
            for &rule in del_rules.iter().sorted().rev() {
                self.rules.remove(rule);
            }
        }
        
        //TODO: any existing algorithms on left-factoring ?
    }
    
    /// Remove all indirect and direct left-recursions
    /// from this grammar.
    fn remove_left_recursions(&mut self) {
        self.remove_duplicates();
        self.left_factoring();
        //TODO: LRNG
        //TODO: LC(LR)
    }

    /// Convert the CNF into GNF by repeatedly substituting
    /// the leftmost nonterminal until every rule starts
    /// with a terminal.
    fn sub_rhs_gnf(&mut self) {
        //TODO: What ordering to choose ?
        //let mut ordering = HashMap::<NonTerminal, usize>::new();
        
        //TODO: substitution of until GNF is met
    }
    
    /// Check whether this grammar is in Greibach Normal Form.
    pub(crate) fn is_gnf(&self) -> bool {
        for rule in &self.rules {
            if !rule.rhs[0].is_terminal() {
                return false;
            }
            
            for i in 1..rule.rhs.len() {
                if !rule.rhs[i].is_non_terminal() {
                    return false;
                }
            }
        }
        
        true
    }

    /// Convert this context-free grammar into Greibach Normal Form.
    pub(crate) fn convert_to_gnf(&mut self) {
        self.new_entrypoint();
        self.isolate_terminals();
        // The grammar specification disallows epsilon rules so we don't have to remove them here
        self.eliminate_unit_rules();
        self.remove_left_recursions();
        self.sub_rhs_gnf();
        self.remove_duplicates();
    }

    /// Access the production rules.
    pub(crate) fn rules(&self) -> &[ProductionRule] {
        &self.rules
    }
    
    /// Access the entrypoint
    pub(crate) fn entrypoint(&self) -> &NonTerminal {
        &self.entrypoint
    }
}

impl Display for NonTerminal {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "<{}>", self.id())
    }
}

impl Display for ContextFreeGrammar {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        writeln!(f, "Entrypoint: {}", self.entrypoint)?;
        writeln!(f, "Rules:")?;

        for (i, rule) in self.rules.iter().enumerate() {
            write!(f, "  #{}: {} -> ", i, rule.lhs)?;

            for var in &rule.rhs {
                match var {
                    Symbol::Terminal(term) => write!(f, "'{}'", self.terminals[term.index()])?,
                    Symbol::NonTerminal(nonterm) => write!(f, "{}", nonterm)?,
                }

                write!(f, " ")?;
            }

            writeln!(f, "")?;
        }

        Ok(())
    }
}
