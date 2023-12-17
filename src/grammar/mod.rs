//! This is the frontend that loads grammars.
//! 
//! Use it like so:
//! ```
//! // Load multiple grammars by joining their rules:
//! let grammar = ContextFreeGrammar::builder()
//!     // Load a grammar in peacock format
//!     .peacock_grammar("my-grammar.json").unwrap()
//!     // Or a grammar in gramatron format
//!     .gramatron_grammar("my-old-grammar.json").unwrap()
//!     // Set the entrypoint
//!     .entrypoint("MY-ENTRYPOINT")
//!     .build().unwrap();
//! ```
//! You can inspect the grammar contents like this:
//! ```
//! // Since a grammar is nothing but a set of rules, traverse the rules
//! for rule in grammar.rules() {
//!     // The left-hand-side (lhs) of a rule is a single non-terminal
//!     println!("lhs = {:?}", rule.lhs());
//! 
//!     // The right-hand-side (rhs) of a rule is a sequence of terminals and non-terminals.
//!     // This is captured in the enum "Symbol".
//!     for symbol in rule.rhs() {
//!         match symbol {
//!             Symbol::Terminal(terminal) => println!("terminal: {}", terminal.content()),
//!             Symbol::NonTerminal(nonterminal) => println!("non-terminal {}", nonterminal.id()),
//!         }
//!     }
//! }
//! ```

mod builder;
mod cfg;

pub use builder::*;
pub use cfg::*;
