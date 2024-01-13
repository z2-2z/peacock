//! Generate a grammar in peacock format.
//! 
//! Use it like so:
//! ```
//! // First, load multiple grammars from disk. This will merge all the rules.
//! let grammar = ContextFreeGrammar::builder()
//!     .peacock_grammar("my-grammar.json").unwrap()
//!     .peacock_grammar("common-definitions.json").unwrap()
//!     .gramatron_grammar("my-old-grammar.json").unwrap()
//!     .build().unwrap();
//! 
//! // Then, create a single new grammar in peacock format.
//! JsonGenerator::new().generate("merged-grammar.json", &grammar);
//! ```

mod generator;

pub use generator::JsonGenerator;
