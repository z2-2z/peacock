//! This library contains everything you need to setup a grammar-based fuzzer.
//! 
//! It consists of
//! - __frontend__: Load grammars of different formats. Currently, the Gramatron and Peacock format are supported.
//! - __backend__: Use the loaded grammar to do whatever you want.
//!   Current backends are
//!   - `C`: Generate a grammar-based mutator in C
//!   - `json`: Convert loaded grammar(s) into peacock format
//! 
//! ## Getting Started
//! The first step always is to load grammars. To do this use the [`ContextFreeGrammar::builder()`](grammar::ContextFreeGrammar::builder) method
//! that will give you access to a [`GrammarBuilder`](grammar::GrammarBuilder) like this:
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
//! Then, you can plug the grammar into one of the provided backends or you can write your own.   
//! If you write your own backend you will most likely need to traverse the grammar:
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
//! And that's it.

//#![deny(missing_docs)]

pub(crate) mod parser;

pub mod error;
pub mod grammar;
pub mod backends;

#[cfg(feature = "components")]
pub mod components;
