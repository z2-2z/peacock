use crate::grammar::{ContextFreeGrammar, GrammarError};

#[test]
fn test_syntax() {
    match ContextFreeGrammar::from_json("src/tests/grammars/syntax.json") {
        Err(GrammarError::InvalidFormat(s)) =>  panic!("{}", s),
        _ => {},
    }
}

#[test]
#[ignore]
fn test_gnf() {
    let mut grammar = ContextFreeGrammar::from_json("src/tests/grammars/gnf.json").unwrap();
    grammar.convert_to_gnf();
    println!("{}", grammar);
}

#[test]
fn test_cycles() {
    match ContextFreeGrammar::from_json("src/tests/grammars/cycles.json") {
        Err(GrammarError::ContainsCycles) => {},
        _ => panic!(),
    }
}

#[test]
fn test_unused() {
    let grammar = ContextFreeGrammar::from_json("src/tests/grammars/unused.json").unwrap();
    println!("{}", grammar);
}
