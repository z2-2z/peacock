use serde::ser::Serialize;
use serde_json::{
    json,
    ser::PrettyFormatter,
    Serializer,
    Value,
};
use std::{
    fs::File,
    io::Write,
    path::Path,
};

use crate::grammar::{
    ContextFreeGrammar,
    Symbol,
};

fn enclosed_in(s: &str, start: char, end: char) -> bool {
    s.len() >= 2 && s.starts_with(start) && s.ends_with(end)
}

fn terminal_string(content: &str) -> String {
    if enclosed_in(content, '<', '>') || enclosed_in(content, '\'', '\'') {
        return format!("'{}'", content);
    }

    content.to_string()
}

/// This is the main struct of the [`json`](crate::backends::json) backend that does all the heavy lifting and generates the grammar.
pub struct JsonGenerator {}

impl JsonGenerator {
    /// Create a new JsonGenerator.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {}
    }

    /// Write the production rules of the supplied `grammar` into the output file `path` in peacock format.
    pub fn generate<P: AsRef<Path>>(self, path: P, grammar: &ContextFreeGrammar) {
        let mut json = json!({});
        let object = json.as_object_mut().unwrap();

        for rule in grammar.rules() {
            let array = object.entry(format!("<{}>", rule.lhs().id())).or_insert_with(|| Value::Array(Vec::new()));
            let array = array.as_array_mut().unwrap();

            let mut insert = Vec::new();
            for symbol in rule.rhs() {
                match symbol {
                    Symbol::Terminal(term) => {
                        insert.push(Value::String(terminal_string(term.content())));
                    },
                    Symbol::NonTerminal(nonterm) => {
                        insert.push(Value::String(format!("<{}>", nonterm.id())));
                    },
                }
            }

            array.push(Value::Array(insert));
        }

        let mut buf = Vec::new();
        let formatter = PrettyFormatter::with_indent(b"    ");
        let mut ser = Serializer::with_formatter(&mut buf, formatter);
        json.serialize(&mut ser).unwrap();

        let mut file = File::create(path).expect("Could not open output file");
        file.write_all(&buf).expect("Could not write to output file");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generator() {
        let cfg = ContextFreeGrammar::builder()
            .gramatron_grammar("test-data/grammars/gramatron.json")
            .unwrap()
            .optimize(false)
            .build()
            .unwrap();
        JsonGenerator::new().generate("/tmp/new.json", &cfg);

        ContextFreeGrammar::builder().peacock_grammar("/tmp/new.json").unwrap().build().unwrap();
    }
}
