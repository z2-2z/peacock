use std::path::Path;
use std::fs::File;
use std::io::BufReader;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::hash::{BuildHasher, Hasher, Hash};

use serde_json as json;
use json_comments::{StripComments, CommentSettings};
use indexmap::IndexMap;
use petgraph::{
    matrix_graph::{MatrixGraph, NodeIndex},
    algo::is_cyclic_directed,
    visit::Bfs as BreadthFirstSearch,
};
use ahash::{
    AHashSet as HashSet,
    AHashMap as HashMap,
    RandomState,
};

/// Name of the non-terminal where generation should start
pub(crate) const ENTRYPOINT: &str = "ENTRYPOINT";

/// Errors that can appear while parsing and converting
/// context free grammars
#[derive(Debug)]
pub(crate) enum GrammarError {
    /// Parsing of the grammar failed
    InvalidFormat(String),
    
    /// The specified grammar contains infinite loops
    ContainsCycles,
}

/// A non-terminal. While non-terminals are identified
/// by names in the grammar we use integers for better
/// efficiency.
#[derive(Clone, Eq, Hash, PartialEq)]
pub(crate) struct NonTerminal(usize);

impl NonTerminal {
    fn id(&self) -> usize {
        self.0
    }
}

/// A terminal that points into the `terminals` array
/// of the context free grammar.
#[derive(Clone, Eq, Hash, PartialEq)]
pub(crate) struct Terminal(usize);

impl Terminal {
    fn index(&self) -> usize {
        self.0
    }
}

/// The set of variables in a context-free grammar is the union of
/// terminals and non-terminals.
#[derive(Clone, Hash)]
pub(crate) enum Variable {
    NonTerminal(NonTerminal),
    Terminal(Terminal),
}

impl Variable {
    fn is_non_terminal(&self) -> bool {
        match self {
            Variable::NonTerminal(_) => true,
            _ => false,
        }
    }
    
    fn is_terminal(&self) -> bool {
        match self {
            Variable::Terminal(_) => true,
            _ => false,
        }
    }
}

/// A single production rule of the context-free grammar.
#[derive(Hash)]
pub(crate) struct ProductionRule {
    lhs: NonTerminal,
    rhs: Vec<Variable>,
}

impl ProductionRule {
    fn fixed_hash(&self) -> u64 {
        let state = RandomState::with_seed(0);
        let mut hasher = state.build_hasher();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

/// A context-free grammar.
pub(crate) struct ContextFreeGrammar {
    terminals: Vec<String>,
    rules: Vec<ProductionRule>,
    entrypoint: NonTerminal,
    nonterminal_cursor: usize,
}

/// A helper for parsing. In the grammar file
/// non-terminals are JSON strings enclosed in '<>' and
/// terminals are strings enclosed in single quotes.
enum VariableString<'a> {
    Terminal(&'a str),
    NonTerminal(&'a str),
}

impl<'a> VariableString<'a> {
    fn parse(value: &'a str) -> Option<Self> {
        if value.starts_with("<") && value.ends_with(">") && value.len() >= 3 {
            Some(VariableString::NonTerminal(&value[1..value.len() - 1]))
        } else if value.starts_with("'") && value.ends_with("'") && value.len() >= 3 {
            Some(VariableString::Terminal(&value[1..value.len() - 1]))
        } else {
            None
        }
    }
}

/// Whenever ContextFreeGrammar::next_nonterminal() is not accessible
/// due to the gosh dang borrow checker reside to this macro instead.
macro_rules! next_nonterminal {
    ($self:expr) => {
        {
            let curr = $self.nonterminal_cursor;
            $self.nonterminal_cursor += 1;
            NonTerminal(curr)
        }
    };
}

impl ContextFreeGrammar {
    /// Creates a context-free grammar from a grammar specified in a JSON file.
    /// For details on grammar syntax see the documentation in the repository.
    pub(crate) fn from_json<P>(path: P) -> Result<Self, GrammarError>
    where
        P: AsRef<Path>,
    {
        let file = File::open(path).unwrap();
        let reader = BufReader::new(file);
        let reader = StripComments::with_settings(
            CommentSettings::c_style(),
            reader
        );
        
        let grammar: json::Value = match json::from_reader(reader) {
            Ok(grammar) => grammar,
            Err(e) => {
                return Err(GrammarError::InvalidFormat(format!("{}", e)));
            },
        };
        
        /* Verify that grammar has correct structure */
        let mut non_terminals = IndexMap::<String, usize>::new();
        let mut terminals = IndexMap::<String, usize>::new();
        
        match &grammar {
            json::Value::Object(map) => {
                for (key, value) in map {
                    let varstring = VariableString::parse(key);
                    
                    if let Some(var) = varstring {
                        match var {
                            VariableString::NonTerminal(name) => {
                                let id = non_terminals.len();
                                non_terminals.insert(name.to_string(), id);
                            },
                            VariableString::Terminal(_) => {
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
                                                match VariableString::parse(variable) {
                                                    None => {
                                                        return Err(GrammarError::InvalidFormat(format!("String on the right-hand side of key {} is neither a terminal nor a non-terminal", key)));
                                                    },
                                                    Some(VariableString::Terminal(term)) => {
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
            let nonterm = match VariableString::parse(key).unwrap() {
                VariableString::Terminal(_) => unreachable!(),
                VariableString::NonTerminal(nonterm) => nonterm,
            };
            let lhs = NonTerminal(*non_terminals.get(nonterm).unwrap());
            
            for value in value.as_array().unwrap() {
                let rhs = match value {
                    json::Value::Array(values) => {
                        let mut variables = Vec::<Variable>::new();
                        
                        for value in values {
                            let variable = match VariableString::parse(value.as_str().unwrap()).unwrap() {
                                VariableString::Terminal(value) => {
                                    Variable::Terminal(Terminal(*terminals.get(value).unwrap()))
                                },
                                VariableString::NonTerminal(value) => if let Some(id) = non_terminals.get(value) {
                                    Variable::NonTerminal(NonTerminal(*id))
                                } else {
                                    return Err(GrammarError::InvalidFormat(format!("Non-terminal '{}' referenced but not defined", value)));
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
        
        'next_rule:
        for rule in &self.rules {
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
            
            let src = if let Some(idx) = nodes.get(&rule.lhs) {
                idx.clone()
            } else {
                let idx = graph.add_node(());
                nodes.insert(rule.lhs.clone(), idx.clone());
                idx
            };
            
            for var in &rule.rhs {
                match var {
                    Variable::NonTerminal(nonterm) => {
                        if terminators.contains(nonterm) {
                            continue;
                        }
                        
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
            let src = if let Some(idx) = nodes.get(&rule.lhs) {
                idx.clone()
            } else {
                let idx = graph.add_node(rule.lhs.id());
                nodes.insert(rule.lhs.clone(), idx.clone());
                idx
            };
            
            for var in &rule.rhs {
                match var {
                    Variable::NonTerminal(nonterm) => {
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
                    Variable::NonTerminal(nonterm) => {
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
                rhs: vec![Variable::NonTerminal(old_entrypoint)],
            });
            
            self.entrypoint = new_entrypoint;
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
                    Variable::Terminal(term) => term.clone(),
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
                
                *var = Variable::NonTerminal(nonterm.clone());
            }
        }
        
        for (term, nonterm) in assoc {
            self.rules.push(ProductionRule {
                lhs: nonterm,
                rhs: vec![Variable::Terminal(term)],
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
                rule.rhs.push(Variable::NonTerminal(nonterm.clone()));
                
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
                    Variable::NonTerminal(rhs) => rhs.clone(),
                    _ => unreachable!(),
                };
                pairs.push((rule.lhs, rhs));
                continue;
            }
            
            i += 1;
        }
        
        /* Check for transitive unit rules: A -> B -> C yields A -> C */
        i = 0;
        
        'outer_loop:
        while i < pairs.len() {
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
    
    /// Convert this context-free grammar into Greibach Normal Form.
    pub(crate) fn convert_to_gnf(&mut self) {
        self.convert_to_cnf();
        //TODO: remove left-recursion
        //TODO
    }
    
    /// Access the production rules
    pub(crate) fn rules(&self) -> &[ProductionRule] {
        &self.rules
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
        
        for rule in &self.rules {
            write!(f, "  {} -> ", rule.lhs)?;
            
            for var in &rule.rhs {
                match var {
                    Variable::Terminal(term) => write!(f, "'{}'", self.terminals[term.index()])?,
                    Variable::NonTerminal(nonterm) => write!(f, "{}", nonterm)?,
                }
                
                write!(f, " ")?;
            }
            
            writeln!(f, "")?;
        }
        
        Ok(())
    }
}
