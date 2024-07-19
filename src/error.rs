//! This module contains various error types.

use std::path::PathBuf;
use thiserror::Error;

/// A ParsingError means that the syntax or format of the provided grammar is invalid.
#[derive(Debug, Error)]
pub struct ParsingError {
    path: PathBuf,
    msg: String,
}

impl ParsingError {
    pub(crate) fn new<P: Into<PathBuf>, S: Into<String>>(path: P, msg: S) -> Self {
        Self {
            path: path.into(),
            msg: msg.into(),
        }
    }
}

impl std::fmt::Display for ParsingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ParsingError in {}: {}", self.path.display(), self.msg)
    }
}

/// A GrammarError represents an error with the content of a grammar.
#[derive(Debug, Error)]
pub enum GrammarError {
    /// The grammar does not contain rules to expand the entrypoint
    #[error("The grammar does not contain an explicit entrypoint: {0}")]
    MissingEntrypoint(String),

    /// The grammar is referencing a non-terminal that has no rules to expand.
    #[error("The non-terminal '{0}' is referenced but never defined")]
    MissingNonTerminal(String),
}
