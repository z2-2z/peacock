//! This library contains everything you need to setup a grammar-based fuzzer.
//! 
//! It consists of
//! - __frontend__: Load grammars of different formats. Currently, the Gramatron and Peacock format are supported.
//! - __backend__: Use the loaded grammar to do whatever you want.
//!   Current backends are
//!   - `C`: Generate a grammar-based mutator in C
//!   - `json`: Convert loaded grammar(s) into peacock format
//! 
//!   but you can easily write your own.
//! - __runtime__: LibAFL components that you can use in your fuzzer to realize grammar-based mutations.
//! 
//! ## Grammars
//! This library supports grammar files in two formats:
//! 1. [Gramatron](https://github.com/HexHive/Gramatron) format for backwards compatibility
//! 2. Its own "peacock format", which is documented in the [README](https://github.com/z2-2z/peacock#how-to-write-grammars) of this project 
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
//! And that's it.
//! 
//! ## Feature Flags
//! - `components`: Include LibAFL components in this library. On by default.
//! - `static-loading`: Activate this if you want to compile the generated C code into the fuzzer. For more details see the
//!   documentation of the `components`.
//! - `debug-codegen`: This affects the C backend and inserts call to printf() at the beginning of every generated function to
//!    help troubleshooting.

#![deny(missing_docs)]

pub(crate) mod parser;

pub mod error;
pub mod grammar;
pub mod backends;

#[cfg(feature = "components")]
pub mod components;
