use std::path::Path;
use std::fs::File;
use std::io::BufReader;

use serde_json as json;
use json_comments::{CommentSettings, StripComments};

use crate::grammar::error::GrammarError;

pub(crate) struct GrammarMerger {
    grammar: json::Value,
}

impl GrammarMerger {
    pub(crate) fn new() -> Self {
        Self {
            grammar: json::json!({}),
        }
    }
    
    pub(crate) fn merge<P>(mut self, path: P) -> Result<Self, GrammarError>
    where
        P: AsRef<Path>,
    {
        let file = File::open(path).unwrap();
        let reader = BufReader::new(file);
        let reader = StripComments::with_settings(CommentSettings::c_style(), reader);

        let new_part: json::Value = match json::from_reader(reader) {
            Ok(new_part) => new_part,
            Err(e) => {
                return Err(GrammarError::InvalidFormat(format!("{}", e)));
            },
        };
        
        let parts = self.grammar.as_object_mut().unwrap();
        
        match new_part {
            json::Value::Object(map) => {
                for (key, value) in map {
                    if parts.contains_key(&key) {
                        return Err(GrammarError::MergeConflict(format!("Two grammars use the same key: {}", key)));
                    }
                    
                    parts.insert(key, value);
                }
            },
            _ => {
                return Err(GrammarError::InvalidFormat("Grammar must be an object".to_string()));
            },
        }
        
        Ok(self)
    }
    
    pub(crate) fn grammar(&self) -> &json::Value {
        &self.grammar
    }
}
