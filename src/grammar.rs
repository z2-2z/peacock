use std::path::Path;
use std::fs::File;
use std::io::BufReader;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::collections::{HashSet, HashMap};

use serde_json as json;
use json_comments::{StripComments, CommentSettings};
use indexmap::IndexMap;
use petgraph::{
    matrix_graph::{MatrixGraph, NodeIndex},
    algo::is_cyclic_directed,
};

/// Name of the non-terminal where generation should start
pub const ENTRYPOINT: &str = "ENTRYPOINT";

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
struct NonTerminal(usize);

impl NonTerminal {
    fn id(&self) -> usize {
        self.0
    }
}

/// A terminal that points into the `terminals` array
/// of the context free grammar.
struct Terminal(usize);

impl Terminal {
    fn index(&self) -> usize {
        self.0
    }
}

/// The set of variables in a context-free grammar is the union of
/// terminals and non-terminals.
enum Variable {
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
}

/// A single production rule of the context-free grammar.
struct ProductionRule {
    lhs: NonTerminal,
    rhs: Vec<Variable>,
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
        
        let ret = ContextFreeGrammar {
            terminals,
            rules,
            entrypoint,
            nonterminal_cursor: nonterminal_cursor + 1,
        };
        
        ret.check_cycles()?;
        
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
    
    /// Convert this context-free grammar into Greibach Normal Form.
    pub(crate) fn convert_to_gnf(&mut self) {
        todo!();
        
        //Ok(())
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
