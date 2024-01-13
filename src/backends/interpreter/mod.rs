//! Generate inputs by interpreting the rules of the grammar.
//! 
//! Use it like so:
//! ```
//! // First, load a grammar from disk
//! let grammar = ContextFreeGrammar::builder()
//!     .peacock_grammar("my-grammar.json").unwrap()
//!     .build().unwrap();
//! 
//! // Then, generate one input and write it to a specified stream.
//! let mut stream = std::io::stdout();
//! GrammarInterpreter::new(&grammar).interpret(&mut stream).unwrap();
//! ```

use std::io::Write;

use crate::{
    backends::C::{LowLevelGrammar, LLSymbol},
    grammar::ContextFreeGrammar,
};

/// The GrammarInterpreter interprets the rules of a grammar to generate inputs.
pub struct GrammarInterpreter {
    grammar: LowLevelGrammar,
    seed: usize,
    stack: Vec<LLSymbol>,
}

impl GrammarInterpreter {
    /// Create a new GrammarInterpreter.
    #[allow(clippy::new_without_default)]
    pub fn new(grammar: &ContextFreeGrammar) -> Self {
        Self {
            grammar: LowLevelGrammar::from_high_level_grammar(grammar),
            seed: 0xDEADBEEF,
            stack: Vec::with_capacity(4096),
        }
    }
    
    /// Seed the RNG of the GrammarInterpreter.
    pub fn seed(&mut self, seed: usize) {
        if seed == 0 {
            self.seed = 0xDEADBEEF;
        } else {
            self.seed = seed;
        }
    }
    
    /// Generate one input and write it to the given output stream `stream`.
    /// Returns the number of bytes written to `stream`.
    pub fn interpret<S: Write>(&mut self, stream: &mut S) -> std::io::Result<usize> {
        let mut generated = 0;
        
        assert!(self.stack.is_empty());
        self.stack.push(LLSymbol::NonTerminal(*self.grammar.entrypoint()));
        
        while let Some(symbol) = self.stack.pop() {
            match symbol {
                LLSymbol::Terminal(term) => {
                    let term = &self.grammar.terminals()[term.id()].as_bytes();
                    generated += term.len();
                    stream.write_all(term)?;
                },
                LLSymbol::NonTerminal(nonterm) => {
                    let rules = self.grammar.rules().get(&nonterm.id()).unwrap();
                    
                    // Inline RNG because of borrow problems
                    let rand = {
                        let mut x = self.seed;
                        x ^= x << 13;
                        x ^= x >> 7;
                        x ^= x << 17;
                        self.seed = x;
                        x
                    };
                    
                    let rule = &rules[rand % rules.len()];
                    
                    for symbol in rule.iter().rev() {
                        self.stack.push(symbol.clone());
                    }
                },
            }
        }
        
        Ok(generated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_interpreter() {
        let cfg = ContextFreeGrammar::builder()
            .gramatron_grammar("test-data/grammars/gramatron.json").unwrap()
            .build().unwrap();
        let mut stdout = std::io::stdout();
        let mut interpreter = GrammarInterpreter::new(&cfg);
        interpreter.seed(1238);
        let len = interpreter.interpret(&mut stdout).unwrap();
        println!();
        println!("Generated {} bytes", len);
    }
}
