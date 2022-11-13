/// Errors that can appear while parsing and converting
/// context free grammars
#[derive(Debug)]
pub enum Error {
    /// Parsing of the grammar failed
    InvalidGrammar(String),

    /// The specified grammar contains infinite loops
    GrammarContainsCycles,

    /// Merging grammars failed
    GrammarMergeConflict(String),
}
