use std::path::Path;
use std::fs::File;
use std::io::BufReader;
use json_comments::{CommentSettings, StripComments};
use serde_json as json;

use crate::{
    ProductionRule,
    error::ParsingError,
};

pub fn parse_json(path: &Path) -> Result<Vec<ProductionRule>, ParsingError> {
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    let reader = StripComments::with_settings(CommentSettings::c_style(), reader);

    let new_part: json::Value = match json::from_reader(reader) {
        Ok(new_part) => new_part,
        Err(e) => {
            return Err(ParsingError::new(
                path,
                "Invalid JSON syntax"
            ));
        },
    };
    
    todo!()
}
