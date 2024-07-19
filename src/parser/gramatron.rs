use serde_json as json;
use std::{
    fs::File,
    io::BufReader,
    path::Path,
};

use crate::{
    error::ParsingError,
    grammar::{
        NonTerminal,
        ProductionRule,
        Symbol,
        Terminal,
    },
};

#[inline]
fn is_whitespace(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\r' | 12 | 11)
}

fn parse_until<F: FnMut(u8) -> bool>(buf: &[u8], mut delim: F) -> &[u8] {
    let mut cursor = 0;

    while cursor < buf.len() {
        if delim(buf[cursor]) {
            break;
        } else {
            cursor += 1;
        }
    }

    &buf[..cursor]
}

fn parse_grammar(value: json::Value) -> Result<Vec<ProductionRule>, String> {
    let mut rules = Vec::new();

    let object = match value {
        json::Value::Object(object) => object,
        _ => return Err("Gramatron grammar must be specified as an object".to_string()),
    };

    for (key, value) in &object {
        let rhs = match value {
            json::Value::Array(rhs) => rhs,
            _ => return Err(format!("Right-hand-side of '{}' must be an array", key)),
        };

        if rhs.is_empty() {
            return Err(format!("Invalid production rule '{}': Must not be empty", key));
        }

        for rule in rhs {
            let rule = match rule.as_str() {
                Some(rule) => rule,
                _ => return Err(format!("Right-hand-side of '{}' must be an array of strings", key)),
            };
            let mut symbols = Vec::new();
            let rule = rule.as_bytes();
            let mut cursor = 0;

            while cursor < rule.len() {
                match &rule[cursor] {
                    b'\'' => {
                        cursor += 1;
                        let content = parse_until(&rule[cursor..], |x| x == b'\'');
                        cursor += content.len() + 1;
                        let content = String::from_utf8(content.to_vec()).unwrap();
                        symbols.push(Symbol::Terminal(Terminal::new(content)));
                    },
                    b'"' => {
                        cursor += 1;
                        let content = parse_until(&rule[cursor..], |x| x == b'"');
                        cursor += content.len() + 1;
                        let content = String::from_utf8(content.to_vec()).unwrap();
                        symbols.push(Symbol::Terminal(Terminal::new(content)));
                    },
                    c => {
                        if is_whitespace(*c) {
                            cursor += 1;
                        } else {
                            let content = parse_until(&rule[cursor..], |x| is_whitespace(x) || x == b'"' || x == b'\'');
                            cursor += content.len();
                            let content = String::from_utf8(content.to_vec()).unwrap();
                            symbols.push(Symbol::NonTerminal(NonTerminal::new(content)));
                        }
                    },
                }
            }

            if symbols.is_empty() {
                return Err(format!("Right-hand-side of '{}' must not contain a string with no tokens", key));
            }

            rules.push(ProductionRule::new(NonTerminal::new(key.clone()), symbols));
        }
    }

    Ok(rules)
}

pub fn parse_json(path: &Path) -> Result<Vec<ProductionRule>, ParsingError> {
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);

    let value: json::Value = match json::from_reader(reader) {
        Ok(value) => value,
        Err(_) => {
            return Err(ParsingError::new(path, "Invalid JSON syntax"));
        },
    };

    parse_grammar(value).map_err(|e| ParsingError::new(path, e))
}
