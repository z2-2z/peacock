use std::path::Path;
use std::fs::File;
use std::io::BufReader;
use json_comments::{CommentSettings, StripComments};
use serde_json as json;

use crate::{
    grammar::{ProductionRule, Symbol, Terminal, NonTerminal},
    error::ParsingError,
};

fn parse_non_terminal(keyword: &str) -> Option<&str> {
    if keyword.len() > 2 && keyword.starts_with('<') && keyword.ends_with('>') {
        Some(&keyword[1..keyword.len() - 1])
    } else {
        None
    }
}

fn parse_terminal(keyword: &str) -> &str {
    if keyword.len() >= 2 && keyword.starts_with('\'') && keyword.ends_with('\'') {
        &keyword[1..keyword.len() - 1]
    } else {
        keyword
    }
}

fn parse_grammar(value: json::Value) -> Result<Vec<ProductionRule>, String> {
    let mut rules = Vec::new();
    
    let object = match value {
        json::Value::Object(object) => object,
        _ => return Err("Peacock grammar must be specified as an object".to_string()),
    };
    
    for (key, value) in &object {
        // LHS must be a non-terminal
        let lhs = match parse_non_terminal(key) {
            Some(lhs) => lhs,
            None => return Err(format!("'{}' is not a valid non-terminal", key)),
        };
        
        // RHS must be an array of an array of strings that are either terminals or non-terminals
        let rhs = match value {
            json::Value::Array(rhs) => rhs,
            _ => return Err(format!("Right-hand-side of '{}' must be an array", key)),
        };
        
        if rhs.is_empty() {
            return Err(format!("Invalid production rule '{}': Must not be empty", key));
        }
        
        for rule in rhs {
            let tokens = match rule {
                json::Value::Array(tokens) => tokens,
                _ => return Err(format!("Right-hand-side of '{}' must be an array of arrays", key)),
            };
            
            if tokens.is_empty() {
                return Err(format!("Invalid production rule '{}': One of its variants is empty", key));
            }
            
            let mut symbols = Vec::new();
            
            for token in tokens {
                let token = match token.as_str() {
                    Some(token) => token,
                    _ => return Err(format!("Right-hand-side of '{}' must be an array of arrays of strings", key)),
                };
                
                if let Some(nonterm) = parse_non_terminal(token) {
                    symbols.push(Symbol::NonTerminal(NonTerminal::new(nonterm)));
                } else {
                    let term = parse_terminal(token);
                    symbols.push(Symbol::Terminal(Terminal::new(term)));
                }
            }
            
            rules.push(ProductionRule::new(
                NonTerminal::new(lhs),
                symbols,
            ));
        }
    }
    
    Ok(rules)
}

pub fn parse_json(path: &Path) -> Result<Vec<ProductionRule>, ParsingError> {
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    let reader = StripComments::with_settings(CommentSettings::c_style(), reader);

    let value: json::Value = match json::from_reader(reader) {
        Ok(value) => value,
        Err(_) => {
            return Err(ParsingError::new(
                path,
                "Invalid JSON syntax"
            ));
        },
    };
    
    parse_grammar(value).map_err(|e| ParsingError::new(path, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_peacock() {
        println!("{:#?}", parse_json(Path::new("test-data/grammars/test-peacock.json")).unwrap());
    }
}
