//! Generate a grammar-based mutator in C.
//! 
//! Use it like so:
//! ```
//! // First, load a grammar from disk
//! let grammar = ContextFreeGrammar::builder()
//!     .peacock_grammar("my-grammar.json").unwrap()
//!     .build().unwrap();
//! 
//! // Then, generate grammar-based mutator code and write it into mutator.c
//! CGenerator::new().generate("mutator.c", grammar);
//! ```

mod formatter;
mod grammar;
mod codegen;

pub use codegen::CGenerator;
