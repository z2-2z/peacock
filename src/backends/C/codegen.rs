use std::fs::File;
use std::path::{Path, PathBuf};

use crate::{
    backends::C::{
        formatter::CFormatter,
        grammar::{LowLevelGrammar, LLSymbol},
    },
    grammar::ContextFreeGrammar,
};

fn rule_has_nonterminals(rule: &[LLSymbol]) -> bool {
    for symbol in rule {
        if matches!(symbol, LLSymbol::NonTerminal(_)) {
            return true;
        }
    }
    
    false
}

fn rules_have_nonterminals(rules: &[Vec<LLSymbol>]) -> bool {
    for rule in rules {
        if rule_has_nonterminals(rule) {
            return true;
        }
    }
    
    false
}

fn rule_has_terminals(rule: &[LLSymbol]) -> bool {
    for symbol in rule {
        if matches!(symbol, LLSymbol::Terminal(_)) {
            return true;
        }
    }
    
    false
}

fn rules_have_terminals(rules: &[Vec<LLSymbol>]) -> bool {
    for rule in rules {
        if rule_has_terminals(rule) {
            return true;
        }
    }
    
    false
}

fn emit_headers(fmt: &mut CFormatter<File>) {
    fmt.write("#include <stddef.h>");
    fmt.write("#include <stdint.h>");
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
    
    fmt.write("#ifndef __clang__");
    fmt.write("#undef __builtin_memcpy_inline");
    fmt.write("#define __builtin_memcpy_inline __builtin_memcpy");
    fmt.write("#endif");
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
    fmt.write("static inline size_t rand (void) {");
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
            fmt.write(format!("if (UNLIKELY(!mutate_seq_nonterm{}(seq, step))) {{", dst.id()));
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
    fmt.write("if (*step >= idx) {");
    fmt.indent();
    fmt.write("if (UNLIKELY(idx >= seq->capacity)) {");
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
    
    fmt.write("*step += 1;");
    fmt.blankline();
    
    emit_mutation_function_rule(rule, fmt);
    
    fmt.write("return 1;");
}

fn emit_mutation_function_multiple(rules: &Vec<Vec<LLSymbol>>, fmt: &mut CFormatter<File>) {
    let have_nonterminals = rules_have_nonterminals(rules);
    
    fmt.write("size_t idx = seq->len;");
    fmt.write("size_t target;");
    fmt.blankline();
    
    if have_nonterminals {
        fmt.write("if (*step < idx) {");
        fmt.indent();
        fmt.write("target = seq->buf[*step];");
        fmt.unindent();
        fmt.write("} else {");
    } else {
        fmt.write("if (*step >= idx) {");
    }
    
    fmt.indent();
    fmt.write("if (UNLIKELY(idx >= seq->capacity)) {");
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
    
    fmt.write("*step += 1;");
    fmt.blankline();
    
    if have_nonterminals {
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
    }
    
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
    
    fmt.write("if (UNLIKELY(!buf || !capacity)) {");
    fmt.indent();
    fmt.write("return 0;");
    fmt.unindent();
    fmt.write("}");
    fmt.blankline();
    
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

fn emit_terminals(grammar: &LowLevelGrammar, fmt: &mut CFormatter<File>) {
    fmt.write("/* Terminals */");
    
    for (i, term) in grammar.terminals().iter().enumerate() {
        let term = term.as_bytes();
        
        if term.len() == 1 || term.len() == 2 || term.len() == 4 || term.len() == 8 {
            continue;
        }
        
        fmt.write(format!("static const unsigned char TERM{}[{}] = {{", i, term.len()));
        fmt.indent();
        
        for chunk in term.chunks(8) {
            let x: Vec<String> = chunk.iter().map(|x| format!("{:#02X},", *x)).collect();
            fmt.write(x.join(" "));
        }
        
        fmt.unindent();
        fmt.write("};");
    }
    
    fmt.blankline();
}

fn emit_serialization_declarations(grammar: &LowLevelGrammar, fmt: &mut CFormatter<File>) {
    fmt.write("/* Forward declarations for serialization functions */");
    
    for nonterm in grammar.rules().keys() {
        fmt.write(format!("static size_t serialize_seq_nonterm{} (size_t*, size_t, unsigned char*, size_t, size_t*);", *nonterm));
    }
    
    fmt.blankline();
}

fn emit_serialization_function_rule(rule: &[LLSymbol], grammar: &LowLevelGrammar, fmt: &mut CFormatter<File>) {
    for symbol in rule {
        match symbol {
            LLSymbol::NonTerminal(nonterm) => {
                fmt.write(format!("len = serialize_seq_nonterm{}(seq, seq_len, out, out_len, step);", nonterm.id()));
                fmt.write("out += len; out_len -= len;");
                fmt.blankline();
            },
            LLSymbol::Terminal(term) => {
                let data = grammar.terminals()[term.id()].as_bytes();
                
                match data.len() {
                    1 => {
                        fmt.write("if (UNLIKELY(out_len < 1)) {");
                        fmt.indent();
                        fmt.write("goto end;");
                        fmt.unindent();
                        fmt.write("}");
                        fmt.write(format!("*out++ = {}; out_len--;", data[0]));
                    },
                    2 => {
                        let short = u16::from_le_bytes(data.try_into().unwrap());
                        fmt.write("if (UNLIKELY(out_len < 2)) {");
                        fmt.indent();
                        fmt.write("goto end;");
                        fmt.unindent();
                        fmt.write("}");
                        fmt.write(format!("*((uint16_t*) out) = {};", short));
                        fmt.write("out += 2; out_len -= 2;");
                    },
                    4 => {
                        let int = u32::from_le_bytes(data.try_into().unwrap());
                        fmt.write("if (UNLIKELY(out_len < 4)) {");
                        fmt.indent();
                        fmt.write("goto end;");
                        fmt.unindent();
                        fmt.write("}");
                        fmt.write(format!("*((uint32_t*) out) = {}U;", int));
                        fmt.write("out += 4; out_len -= 4;");
                    },
                    8 => {
                        let long = u64::from_le_bytes(data.try_into().unwrap());
                        fmt.write("if (UNLIKELY(out_len < 8)) {");
                        fmt.indent();
                        fmt.write("goto end;");
                        fmt.unindent();
                        fmt.write("}");
                        fmt.write(format!("*((uint64_t*) out) = {}ULL;", long));
                        fmt.write("out += 8; out_len -= 8;");
                    },
                    _ => {
                        fmt.write(format!("if (UNLIKELY(out_len < sizeof(TERM{}))) {{", term.id()));
                        fmt.indent();
                        fmt.write("goto end;");
                        fmt.unindent();
                        fmt.write("}");
                        fmt.write(format!("__builtin_memcpy_inline(out, TERM{0}, sizeof(TERM{0}));", term.id()));
                        fmt.write(format!("out += sizeof(TERM{0}); out_len -= sizeof(TERM{0});", term.id()));
                        fmt.blankline();
                    },
                }
            },
        }
    }
}

fn emit_serialization_function_single(rule: &[LLSymbol], grammar: &LowLevelGrammar, fmt: &mut CFormatter<File>) {
    let has_nonterminals = rule_has_nonterminals(rule);
    
    if !has_nonterminals {
        fmt.write("(void) seq;");
        fmt.blankline();
    }
    
    fmt.write("if (UNLIKELY(*step >= seq_len)) {");
    fmt.indent();
    fmt.write("return 0;");
    fmt.unindent();
    fmt.write("}");
    fmt.blankline();
    
    if has_nonterminals {
        fmt.write("size_t len;");
    }
    
    fmt.write("unsigned char* original_out = out;");
    fmt.write("*step += 1;");
    fmt.blankline();
    
    emit_serialization_function_rule(rule, grammar, fmt);
    
    if rule_has_terminals(rule) {
        fmt.write("end:");
    }
    fmt.write("return (size_t) (out - original_out);");
}

fn emit_serialization_function_multiple(rules: &[Vec<LLSymbol>], grammar: &LowLevelGrammar, fmt: &mut CFormatter<File>) {
    fmt.write("if (UNLIKELY(*step >= seq_len)) {");
    fmt.indent();
    fmt.write("return 0;");
    fmt.unindent();
    fmt.write("}");
    fmt.blankline();
    
    if rules_have_nonterminals(rules) {
        fmt.write("size_t len;");
    }
    
    fmt.write("unsigned char* original_out = out;");
    fmt.write("size_t target = seq[*step];");
    fmt.write("*step += 1;");
    fmt.blankline();
    
    fmt.write("switch (target) {");
    fmt.indent();
    
    for (i, rule) in rules.iter().enumerate() {
        fmt.write(format!("case {}: {{", i));
        fmt.indent();
        
        emit_serialization_function_rule(rule, grammar, fmt);
        
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
    
    if rules_have_terminals(rules) {
        fmt.write("end:");
    }
    fmt.write("return (size_t) (out - original_out);");
}

fn emit_serialization_function(nonterm: usize, rules: &Vec<Vec<LLSymbol>>, grammar: &LowLevelGrammar, fmt: &mut CFormatter<File>) {
    fmt.write(format!("// This is the serialization function for non-terminal {:?}", grammar.nonterminals()[nonterm]));
    fmt.write(format!("static size_t serialize_seq_nonterm{} (size_t* seq, size_t seq_len, unsigned char* out, size_t out_len, size_t* step) {{", nonterm));
    fmt.indent();
    
    if rules.is_empty() {
        unreachable!()
    } else if rules.len() == 1 {
        emit_serialization_function_single(&rules[0], grammar, fmt);
    } else {
        emit_serialization_function_multiple(rules, grammar, fmt);
    }
    
    fmt.unindent();
    fmt.write("}");
    fmt.blankline();
}

fn emit_serialization_entrypoint(grammar: &LowLevelGrammar, fmt: &mut CFormatter<File>) {
    fmt.write("size_t serialize_sequence (size_t* seq, size_t seq_len, unsigned char* out, size_t out_len) {");
    fmt.indent();
    
    fmt.write("if (UNLIKELY(!seq || !seq_len || !out || !out_len)) {");
    fmt.indent();
    fmt.write("return 0;");
    fmt.unindent();
    fmt.write("}");
    fmt.blankline();
    
    fmt.write("size_t step = 0;");
    fmt.blankline();
    
    fmt.write(format!("return serialize_seq_nonterm{}(seq, seq_len, out, out_len, &step);", grammar.entrypoint().id()));
    fmt.unindent();
    fmt.write("}");
}

fn emit_serialization_code(grammar: &LowLevelGrammar, fmt: &mut CFormatter<File>) {
    emit_terminals(grammar, fmt);
    emit_serialization_declarations(grammar, fmt);
    
    for (nonterm, rules) in grammar.rules() {
        emit_serialization_function(*nonterm, rules, grammar, fmt);
    }
    
    emit_serialization_entrypoint(grammar, fmt);
}

pub struct CGenerator {
    outfile: PathBuf,
    //TODO: optional header file
    //TODO: prefix
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
        emit_serialization_code(&grammar, &mut formatter);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_generator() {
        let cfg = ContextFreeGrammar::builder()
            .gramatron_grammar("test-data/grammars/gramatron.json").unwrap()
            .build().unwrap();
        CGenerator::new("/tmp/out.c").generate(cfg);
    }
}
