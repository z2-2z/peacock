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
//! Then, you can plug the grammar into one of the provided backends:
//! ```
//! backends::C::CGenerator::new().generate("output-file.c", grammar);
//! // or
//! backends::json::JsonGenerator::new().generate("output-file.json", grammar);
//! ```

#![deny(missing_docs)]

pub(crate) mod parser;

pub mod error;
pub mod grammar;
pub mod backends;

#[cfg(feature = "components")]
pub mod components;
