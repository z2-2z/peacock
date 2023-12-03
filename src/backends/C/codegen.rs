use std::fs::File;
use std::path::{Path, PathBuf};

use crate::{
    backends::C::{
        formatter::CFormatter,
        grammar::{LowLevelGrammar, LLSymbol},
    },
    grammar::ContextFreeGrammar,
};

fn emit_headers(fmt: &mut CFormatter<File>) {
    fmt.write("#include <stddef.h>");
    fmt.blankline();
}

fn emit_macros(fmt: &mut CFormatter<File>) {
    fmt.write("/* Helper Macros */");
    
    fmt.write("#undef THREAD_LOCAL");
    fmt.write("#ifdef MULTITHREADING");
    fmt.write("#define THREAD_LOCAL __thread");
    fmt.write("#else");
    fmt.write("#define THREAD_LOCAL");
    fmt.write("#endif");
    fmt.blankline();
    
    fmt.write("#undef UNLIKELY");
    fmt.write("#define UNLIKELY(x) __builtin_expect(!!(x), 0)");
    fmt.write("#undef LIKELY");
    fmt.write("#define LIKELY(x) __builtin_expect(!!(x), 1)");
    fmt.blankline();
}

fn emit_rand(fmt: &mut CFormatter<File>) {
    fmt.write("/* RNG */");
    
    fmt.write("#ifndef SEED");
    fmt.write(" #define SEED 0x35c6be9ba2548264");
    fmt.write("#endif");
    fmt.blankline();
    
    fmt.write("static THREAD_LOCAL size_t rand_state = SEED;");
    fmt.blankline();
    
    fmt.write("#ifndef DISABLE_rand");
    fmt.write("static size_t rand (void) {");
    fmt.indent();
    fmt.write("size_t x = rand_state;");
    fmt.write("x ^= x << 13;");
    fmt.write("x ^= x >> 7;");
    fmt.write("x ^= x << 17;");
    fmt.write("return rand_state = x;");
    fmt.unindent();
    fmt.write("}");
    fmt.write("#else");
    fmt.write("size_t rand (void);");
    fmt.write("#endif");
    fmt.blankline();
    
    //TODO: seeding function
}

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

fn emit_mutation_function_rule(rule: &[LLSymbol], fmt: &mut CFormatter<File>) {
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
}

fn emit_mutation_function_single(rule: &[LLSymbol], fmt: &mut CFormatter<File>) {
    fmt.write("size_t idx = seq->len;");
    fmt.blankline();
    fmt.write("if (*step < idx) {");
    fmt.indent();
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
    fmt.write("seq->buf[idx] = 0;");
    fmt.write("seq->len = idx + 1;");
    fmt.unindent();
    fmt.write("}");
    fmt.blankline();
    
    emit_mutation_function_rule(rule, fmt);
    
    fmt.write("return 1;");
}

fn emit_mutation_function_multiple(rules: &Vec<Vec<LLSymbol>>, fmt: &mut CFormatter<File>) {
    fmt.write("size_t idx = seq->len;");
    fmt.write("size_t target;");
    fmt.blankline();
    fmt.write("if (*step < idx) {");
    fmt.indent();
    fmt.write("target = seq->buf[*step];");
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
        
        emit_mutation_function_rule(rule, fmt);
        
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
}

fn emit_mutation_function(nonterm: usize, rules: &Vec<Vec<LLSymbol>>, grammar: &LowLevelGrammar, fmt: &mut CFormatter<File>) {
    fmt.write(format!("// This is the sequence mutation function for non-terminal {:?}", grammar.nonterminals()[nonterm]));
    fmt.write(format!("static int mutate_seq_nonterm{} (Sequence* seq, size_t* step) {{", nonterm));
    fmt.indent();
    
    if rules.is_empty() {
        unreachable!()
    } else if rules.len() == 1 {
        emit_mutation_function_single(&rules[0], fmt);
    } else {
        emit_mutation_function_multiple(rules, fmt);
    }
    
    fmt.unindent();
    fmt.write("}");
    fmt.blankline();
}

fn emit_mutation_entrypoint(grammar: &LowLevelGrammar, fmt: &mut CFormatter<File>) {
    fmt.write("size_t mutate_sequence (void* buf, size_t len, size_t capacity) {");
    fmt.indent();
    
    fmt.write("Sequence seq = {");
    fmt.indent();
    fmt.write(".buf = (size_t*) buf,");
    fmt.write(".len = len,");
    fmt.write(".capacity = capacity,");
    fmt.unindent();
    fmt.write("};");
    
    fmt.write("size_t step = 0;");
    fmt.blankline();
    
    fmt.write(format!("mutate_seq_nonterm{}(&seq, &step);", grammar.entrypoint().id()));
    fmt.blankline();
    
    fmt.write("return seq.len;");
    
    fmt.unindent();
    fmt.write("}");
}

fn emit_mutation_code(grammar: &LowLevelGrammar, fmt: &mut CFormatter<File>) {
    emit_mutation_types(fmt);
    
    emit_mutation_declarations(grammar, fmt);
    
    for (nonterm, rules) in grammar.rules() {
        emit_mutation_function(*nonterm, rules, grammar, fmt);
    }
    
    emit_mutation_entrypoint(grammar, fmt);
}

pub struct CGenerator {
    outfile: PathBuf,
    //TODO: optional header file
    // prefix
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
        
        emit_headers(&mut formatter);
        emit_macros(&mut formatter);
        emit_rand(&mut formatter);
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
