
/// Errors that can appear while parsing and converting
/// context free grammars
#[derive(Debug)]
pub(crate) enum GrammarError {
    /// Parsing of the grammar failed
    InvalidFormat(String),

    /// The specified grammar contains infinite loops
    ContainsCycles,
    
    /// Merging grammars failed
    MergeConflict(String),
}
