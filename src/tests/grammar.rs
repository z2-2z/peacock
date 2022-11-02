use crate::grammar::{ContextFreeGrammar, GrammarError};

#[test]
fn test_syntax() {
    match ContextFreeGrammar::from_json("src/tests/grammars/syntax.json") {
        Err(GrammarError::InvalidFormat(s)) =>  panic!("{}", s),
        _ => {},
    }
}

#[test]
fn test_cnf() {
    let mut grammar = ContextFreeGrammar::from_json("src/tests/grammars/cnf.json").unwrap();
    grammar.convert_to_cnf();
    assert!(grammar.is_cnf());
    
    let old_len = grammar.rules().len();
    grammar.convert_to_cnf();
    assert_eq!(old_len, grammar.rules().len());
    
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
