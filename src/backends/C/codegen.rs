use std::fs::File;
use std::path::{Path, PathBuf};

use crate::{
    backends::C::{
        formatter::CFormatter,
        grammar::{LowLevelGrammar, LLSymbol},
    },
    grammar::ContextFreeGrammar,
};

fn emit_mutation_types(fmt: &mut CFormatter<File>) {
    fmt.write("// Used by mutation functions to represent a sequence of non-terminals");
    fmt.write("typedef struct {");
    fmt.indent();
    fmt.write("size_t* buf;");
    fmt.write("size_t len;");
    fmt.write("size_t capacity;");
    fmt.unindent();
    fmt.write("} Sequence;");
    fmt.blankline();
}

fn emit_mutation_declarations(grammar: &LowLevelGrammar, fmt: &mut CFormatter<File>) {
    fmt.write("/* Forward declarations for sequence mutation functions */");
    
    for nonterm in grammar.rules().keys() {
        fmt.write(format!("static int mutate_seq_nonterm{} (Sequence*, size_t*);", *nonterm));
    }
    
    fmt.blankline();
}

fn emit_mutation_function(nonterm: usize, rules: &Vec<Vec<LLSymbol>>, grammar: &LowLevelGrammar, fmt: &mut CFormatter<File>) {
    fmt.write(format!("// This is the sequence mutation function for non-terminal {:?}", grammar.nonterminals()[nonterm]));
    fmt.write(format!("static int mutate_seq_nonterm{} (Sequence* seq, size_t* step) {{", nonterm));
    fmt.indent();
    
    fmt.write("size_t idx = seq->len;");
    fmt.write("size_t target;");
    fmt.blankline();
    fmt.write("if (*step < idx) {");
    fmt.indent();
    fmt.write("target = seq->buf[step];");
    fmt.write("*step += 1;");
    fmt.unindent();
    fmt.write("} else {");
    fmt.indent();
    fmt.write("if (idx >= seq->capacity) {");
    fmt.indent();
    fmt.write("return 0;");
    fmt.unindent();
    fmt.write("}");
    fmt.blankline();
    fmt.write(format!("target = rand() % {};", rules.len()));
    fmt.write("seq->buf[idx] = target;");
    fmt.write("seq->len = idx + 1;");
    fmt.unindent();
    fmt.write("}");
    fmt.blankline();
    
    fmt.write("switch (target) {");
    fmt.indent();
    
    for (i, rule) in rules.iter().enumerate() {
        fmt.write(format!("case {}: {{", i));
        fmt.indent();
        
        for symbol in rule {
            if let LLSymbol::NonTerminal(dst) = symbol {
                fmt.write(format!("if (!mutate_seq_nonterm{}(seq, step)) {{", dst.id()));
                fmt.indent();
                fmt.write("return 0;");
                fmt.unindent();
                fmt.write("}");
                fmt.blankline();
            }
        }
        
        fmt.write("break;");
        fmt.unindent();
        fmt.write("}");
    }
    
    fmt.write("default: {");
    fmt.indent();
    fmt.write("__builtin_unreachable();");
    fmt.unindent();
    fmt.write("}");
    
    fmt.unindent();
    fmt.write("}");
    fmt.blankline();
    fmt.write("return 1;");
    
    fmt.unindent();
    fmt.write("}");
    fmt.blankline();
}

fn emit_mutation_code(grammar: &LowLevelGrammar, fmt: &mut CFormatter<File>) {
    emit_mutation_types(fmt);
    
    emit_mutation_declarations(grammar, fmt);
    
    for (nonterm, rules) in grammar.rules() {
        emit_mutation_function(*nonterm, rules, grammar, fmt);
    }
}

pub struct CGenerator {
    outfile: PathBuf,
    //TODO: optional header file
}

impl CGenerator {
    pub fn new<P: AsRef<Path>>(outfile: P) -> Self {
        Self {
            outfile: outfile.as_ref().to_path_buf(),
        }
    }
    
    pub fn generate(self, grammar: ContextFreeGrammar) {
        let grammar = LowLevelGrammar::from_high_level_grammar(grammar);
        let outfile = File::create(self.outfile).expect("Could not create outfile");
        let mut formatter = CFormatter::new(outfile);
        
        emit_mutation_code(&grammar, &mut formatter);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_generator() {
        let cfg = ContextFreeGrammar::builder()
            .peacock_grammar("test-data/grammars/unit_rules.json").unwrap()
            .build().unwrap();
        CGenerator::new("/tmp/out.c").generate(cfg);
    }
}
