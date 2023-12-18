use std::fs::File;
use std::path::Path;
use std::io::Write;
use itertools::Itertools;

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

fn emit_includes(fmt: &mut CFormatter<File>) {
    #[cfg(debug_codegen)]
    fmt.write("#include <stdio.h>");
    
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
    
    fmt.write("#ifndef __clang__");
    fmt.write("#undef __builtin_memcpy_inline");
    fmt.write("#define __builtin_memcpy_inline __builtin_memcpy");
    fmt.write("#endif");
    fmt.blankline();
    
    fmt.write("#undef EXPORT_FUNCTION");
    fmt.write("#ifdef MAKE_VISIBLE");
    fmt.write("#define EXPORT_FUNCTION __attribute__((visibility (\"default\")))");
    fmt.write("#else");
    fmt.write("#define EXPORT_FUNCTION");
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
    
    fmt.write("#ifndef DISABLE_seed");
    fmt.write("EXPORT_FUNCTION");
    fmt.write("void seed_generator (size_t new_seed) {");
    fmt.indent();
    fmt.write("if (!new_seed) {");
    fmt.indent();
    fmt.write("new_seed = 0xDEADBEEF;");
    fmt.unindent();
    fmt.write("}");
    fmt.blankline();
    fmt.write("rand_state = new_seed;");
    fmt.unindent();
    fmt.write("}");
    fmt.write("#else");
    fmt.write("void seed_generator (size_t);");
    fmt.write("#endif");
    fmt.blankline();
}

fn emit_types(fmt: &mut CFormatter<File>) {
    fmt.write("// Used to represent a sequence of rules");
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
        fmt.write(format!("static int mutate_seq_nonterm{} (Sequence* const, size_t* const);", *nonterm));
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

fn emit_mutation_function_multiple(rules: &[Vec<LLSymbol>], fmt: &mut CFormatter<File>) {
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

fn emit_mutation_function(nonterm: usize, rules: &[Vec<LLSymbol>], grammar: &LowLevelGrammar, fmt: &mut CFormatter<File>) {
    fmt.write(format!("// This is the sequence mutation function for non-terminal {:?}", grammar.nonterminals()[nonterm]));
    fmt.write(format!("static int mutate_seq_nonterm{} (Sequence* const seq, size_t* const step) {{", nonterm));
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
    fmt.write("EXPORT_FUNCTION");
    fmt.write("size_t mutate_sequence (size_t* buf, size_t len, const size_t capacity) {");
    fmt.indent();
    
    #[cfg(debug_codegen)]
    {
        fmt.write("printf(\"Calling mutate_sequence(%p, %lu, %lu)\\n\", buf, len, capacity);");
    }
    
    fmt.write("if (UNLIKELY(!buf || !capacity)) {");
    fmt.indent();
    fmt.write("return 0;");
    fmt.unindent();
    fmt.write("}");
    
    fmt.write("Sequence seq = {");
    fmt.indent();
    fmt.write(".buf = buf,");
    fmt.write(".len = len,");
    fmt.write(".capacity = capacity,");
    fmt.unindent();
    fmt.write("};");
    
    fmt.write("size_t step = 0;");
    fmt.write(format!("mutate_seq_nonterm{}(&seq, &step);", grammar.entrypoint().id()));
    fmt.write("return seq.len;");
    
    fmt.unindent();
    fmt.write("}");
    fmt.blankline();
}

fn emit_mutation_code(grammar: &LowLevelGrammar, fmt: &mut CFormatter<File>) {
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
        fmt.write(format!("static size_t serialize_seq_nonterm{} (const size_t* const, const size_t, unsigned char*, size_t, size_t* const);", *nonterm));
    }
    
    fmt.blankline();
}

fn emit_serialization_function_rule(rule: &[LLSymbol], fmt: &mut CFormatter<File>) {
    for symbol in rule {
        match symbol {
            LLSymbol::NonTerminal(nonterm) => {
                fmt.write(format!("len = serialize_seq_nonterm{}(seq, seq_len, out, out_len, step);", nonterm.id()));
                fmt.write("out += len; out_len -= len;");
                fmt.blankline();
            },
            LLSymbol::Terminal(term) => {
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
    }
}

fn emit_serialization_function_single(rule: &[LLSymbol], fmt: &mut CFormatter<File>) {
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
    
    emit_serialization_function_rule(rule, fmt);
    
    if rule_has_terminals(rule) {
        fmt.write("end:");
    }
    fmt.write("return (size_t) (out - original_out);");
}

fn emit_serialization_function_multiple(rules: &[Vec<LLSymbol>], fmt: &mut CFormatter<File>) {
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
        
        emit_serialization_function_rule(rule, fmt);
        
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

fn emit_serialization_function(nonterm: usize, rules: &[Vec<LLSymbol>], grammar: &LowLevelGrammar, fmt: &mut CFormatter<File>) {
    fmt.write(format!("// This is the serialization function for non-terminal {:?}", grammar.nonterminals()[nonterm]));
    fmt.write(format!("static size_t serialize_seq_nonterm{} (const size_t* const seq, const size_t seq_len, unsigned char* out, size_t out_len, size_t* const step) {{", nonterm));
    fmt.indent();
    
    #[cfg(debug_codegen)]
    {
        fmt.write(format!("printf(\"Serializing %s (%lu/%lu)\\n\", {:?}, *step + 1, seq_len);", grammar.nonterminals()[nonterm]));
    }
    
    if rules.is_empty() {
        unreachable!()
    } else if rules.len() == 1 {
        emit_serialization_function_single(&rules[0], fmt);
    } else {
        emit_serialization_function_multiple(rules, fmt);
    }
    
    fmt.unindent();
    fmt.write("}");
    fmt.blankline();
}

fn emit_serialization_entrypoint(grammar: &LowLevelGrammar, fmt: &mut CFormatter<File>) {
    fmt.write("EXPORT_FUNCTION");
    fmt.write("size_t serialize_sequence (const size_t* seq, const size_t seq_len, unsigned char* out, const size_t out_len) {");
    fmt.indent();
    
    fmt.write("if (UNLIKELY(!seq || !seq_len || !out || !out_len)) {");
    fmt.indent();
    fmt.write("return 0;");
    fmt.unindent();
    fmt.write("}");
    
    fmt.write("size_t step = 0;");
    fmt.write(format!("return serialize_seq_nonterm{}(seq, seq_len, out, out_len, &step);", grammar.entrypoint().id()));
    fmt.unindent();
    fmt.write("}");
    fmt.blankline();
}

fn emit_serialization_code(grammar: &LowLevelGrammar, fmt: &mut CFormatter<File>) {
    emit_terminals(grammar, fmt);
    emit_serialization_declarations(grammar, fmt);
    
    for (nonterm, rules) in grammar.rules() {
        emit_serialization_function(*nonterm, rules, grammar, fmt);
    }
    
    emit_serialization_entrypoint(grammar, fmt);
}

fn emit_header(mut outfile: File, mutations: bool, serializations: bool, unparsing: bool) -> Result<(), std::io::Error> {
    write!(
        &mut outfile,
        "
#ifndef __PEACOCK_GENERATOR_H
#define __PEACOCK_GENERATOR_H

#include <stddef.h>
")?;
    
    if mutations {
        writeln!(&mut outfile, "size_t mutate_sequence (size_t* buf, size_t len, const size_t capacity);")?;
    }

    if serializations {
        writeln!(&mut outfile, "size_t serialize_sequence (const size_t* seq, const size_t seq_len, unsigned char* out, const size_t out_len);")?;
    }

    if unparsing {
        writeln!(&mut outfile, "size_t unparse_sequence (size_t* seq_buf, const size_t seq_capacity, const unsigned char* input, const size_t input_len);")?;
    }
    
    write!(
        &mut outfile,
        "
void seed_generator (size_t new_seed);


#endif /* __PEACOCK_GENERATOR_H */
"
    )?;
    
    Ok(())
}

fn emit_unparsing_declarations(grammar: &LowLevelGrammar, fmt: &mut CFormatter<File>) {
    fmt.write("/* Forward declarations for unparsing functions */");
    
    for nonterm in grammar.rules().keys() {
        fmt.write(format!("static int unparse_seq_nonterm{} (Sequence* const, const unsigned char* const, const size_t, size_t* const);", *nonterm));
    }
    
    fmt.blankline();
}

fn emit_unparsing_function(nonterm: usize, rules: &[Vec<LLSymbol>], grammar: &LowLevelGrammar, fmt: &mut CFormatter<File>) {
    fmt.write(format!("// This is the unparsing function for non-terminal {:?}", grammar.nonterminals()[nonterm]));
    fmt.write(format!("static int unparse_seq_nonterm{} (Sequence* const seq, const unsigned char* const input, const size_t input_len, size_t* const cursor) {{", nonterm));
    fmt.indent();
    
    fmt.write("size_t seq_idx = seq->len;");
    fmt.blankline();
    fmt.write("if (UNLIKELY(seq_idx >= seq->capacity)) {");
    fmt.indent();
    fmt.write("return 0;");
    fmt.unindent();
    fmt.write("}");
    fmt.blankline();
    
    fmt.write("size_t target_cursor = 0;");
    fmt.write("size_t target_id = (size_t) -1LL;");
    fmt.write("size_t target_seq_len = seq_idx;");
    fmt.blankline();
    
    for (i, rule) in rules.iter().enumerate().sorted_by(|(_, a), (_, b)| b.len().cmp(&a.len())) {
        fmt.write(format!("// Rule #{}", i));
        fmt.write("do {");
        fmt.indent();
        fmt.write("seq->len = seq_idx + 1;");
        fmt.write("size_t tmp_cursor = *cursor;");
        fmt.blankline();
        
        for symbol in rule {
            match symbol {
                LLSymbol::Terminal(term) => {
                    fmt.write(format!("if (UNLIKELY(input_len - tmp_cursor < sizeof(TERM{0})) || __builtin_memcmp(&input[tmp_cursor], TERM{0}, sizeof(TERM{0})) != 0) {{", term.id()));
                    fmt.indent();
                    fmt.write("break;");
                    fmt.unindent();
                    fmt.write("}");
                    fmt.write(format!("tmp_cursor += sizeof(TERM{0});", term.id()));
                    fmt.blankline();
                },
                LLSymbol::NonTerminal(nonterm) => {
                    fmt.write(format!("if (!unparse_seq_nonterm{}(seq, input, input_len, &tmp_cursor)) {{", nonterm.id()));
                    fmt.indent();
                    fmt.write("break;");
                    fmt.unindent();
                    fmt.write("}");
                    fmt.blankline();
                },
            }
        }
        
        fmt.write("if (tmp_cursor > target_cursor) {");
        fmt.indent();
        fmt.write(format!("target_id = {};", i));
        fmt.write("target_cursor = tmp_cursor;");
        fmt.write("target_seq_len = seq->len;");
        fmt.unindent();
        fmt.write("}");
        
        fmt.unindent();
        fmt.write("} while (0);");
        fmt.blankline();
    }
    
    fmt.write("seq->len = target_seq_len;");
    fmt.blankline();
    
    fmt.write(format!("if (target_id < {}) {{", rules.len()));
    fmt.indent();
    fmt.write("*cursor = target_cursor;");
    fmt.write("seq->buf[seq_idx] = target_id;");
    fmt.write("return 1;");
    fmt.unindent();
    fmt.write("} else {");
    fmt.indent();
    fmt.write("return 0;");
    fmt.unindent();
    fmt.write("}");
    
    fmt.unindent();
    fmt.write("}");
    fmt.blankline();
}

fn emit_unparsing_entrypoint(grammar: &LowLevelGrammar, fmt: &mut CFormatter<File>) {
    fmt.write("EXPORT_FUNCTION");
    fmt.write("size_t unparse_sequence (size_t* seq_buf, const size_t seq_capacity, const unsigned char* input, const size_t input_len) {");
    fmt.indent();
    
    fmt.write("if (UNLIKELY(!seq_buf || !seq_capacity || !input || !input_len)) {");
    fmt.indent();
    fmt.write("return 0;");
    fmt.unindent();
    fmt.write("}");
    
    fmt.write("Sequence seq = {");
    fmt.indent();
    fmt.write(".buf = seq_buf,");
    fmt.write(".len = 0,");
    fmt.write(".capacity = seq_capacity,");
    fmt.unindent();
    fmt.write("};");
    fmt.write("size_t cursor = 0;");
    fmt.write(format!("if (!unparse_seq_nonterm{}(&seq, input, input_len, &cursor)) {{", grammar.entrypoint().id()));
    fmt.indent();
    fmt.write("return 0;");
    fmt.unindent();
    fmt.write("} else { ");
    fmt.indent();
    fmt.write("return seq.len;");
    fmt.unindent();
    fmt.write("}");
    fmt.unindent();
    fmt.write("}");
    fmt.blankline();
}

fn emit_unparsing_code(grammar: &LowLevelGrammar, fmt: &mut CFormatter<File>) {
    emit_unparsing_declarations(grammar, fmt);
    
    for (nonterm, rules) in grammar.rules() {
        emit_unparsing_function(*nonterm, rules, grammar, fmt);
    }
    
    emit_unparsing_entrypoint(grammar, fmt);
}

/// This is the main struct of the [`C`](crate::backends::C) backend that does all the heavy lifting and generates the code.
/// 
/// For documentation of the generated C code see the [README](https://github.com/z2-2z/peacock#c-api-documentation) of this project.
pub struct CGenerator {
    header: bool,
    mutations: bool,
    serializations: bool,
    unparsing: bool,
}

impl CGenerator {
    /// Create a new CGenerator.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            header: true,
            mutations: true,
            serializations: true,
            unparsing: true,
        }
    }
    
    /// Also generate a .h file with all the definitions of the public C API of the generated code.
    /// 
    /// Default: `true`
    pub fn generate_header(mut self, flag: bool) -> Self {
        self.header = flag;
        self
    }
    
    /// Emit code that realizes the mutation of an automaton walk.
    /// 
    /// Default: `true`
    pub fn emit_mutation_procedure(mut self, flag: bool) -> Self {
        self.mutations = flag;
        self
    }
    
    /// Emit code that realizes the serialization of automaton walks into human-readable output.
    /// 
    /// Default: `true`
    pub fn emit_serialization_procedure(mut self, flag: bool) -> Self {
        self.serializations = flag;
        self
    }
    
    /// Emit code that realizes the unparsing of user inputs into automaton walks.
    /// 
    /// Default: `true`
    pub fn emit_unparsing_procedure(mut self, flag: bool) -> Self {
        self.unparsing = flag;
        self
    }
    
    /// Generate the C code for the given grammar `grammar` and write it to `output`.
    pub fn generate<P: AsRef<Path>>(self, output: P, grammar: ContextFreeGrammar) {
        let grammar = LowLevelGrammar::from_high_level_grammar(grammar);
        let outfile = File::create(output.as_ref()).expect("Could not create source file");
        let mut formatter = CFormatter::new(outfile);
        
        emit_includes(&mut formatter);
        emit_macros(&mut formatter);
        emit_types(&mut formatter);
        emit_rand(&mut formatter);
        
        if self.mutations {
            emit_mutation_code(&grammar, &mut formatter);
        }
        
        if self.serializations {
            emit_serialization_code(&grammar, &mut formatter);
        }
        
        if self.unparsing {
            emit_unparsing_code(&grammar, &mut formatter);
        }
        
        if self.header {
            let mut outfile = output.as_ref().to_path_buf();
            outfile.set_extension("h");
            let outfile = File::create(outfile).expect("Could not create header file");
            emit_header(outfile, self.mutations, self.serializations, self.unparsing).expect("Could not write to header file");
        }
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
