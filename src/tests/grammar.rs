use crate::grammar::{
    cfg::ContextFreeGrammar,
    error::GrammarError,
    merge::GrammarMerger,
};

#[test]
fn test_syntax() {
    let merger = GrammarMerger::new()
                 .merge("src/tests/grammars/syntax.json").unwrap();
    
    match ContextFreeGrammar::from_json(merger.grammar()) {
        Err(GrammarError::InvalidFormat(s)) => panic!("{}", s),
        _ => {},
    }
}

#[test]
fn test_cnf() {
    let merger = GrammarMerger::new()
                 .merge("src/tests/grammars/cnf.json").unwrap();
    
    let mut grammar = ContextFreeGrammar::from_json(merger.grammar()).unwrap();
    grammar.convert_to_cnf();
    assert!(grammar.is_cnf());

    let old_len = grammar.rules().len();
    grammar.convert_to_cnf();
    assert_eq!(old_len, grammar.rules().len());

    println!("{}", grammar);
}

#[test]
fn test_cycles() {
    let merger = GrammarMerger::new()
                 .merge("src/tests/grammars/cycles.json").unwrap();
    
    match ContextFreeGrammar::from_json(merger.grammar()) {
        Err(GrammarError::ContainsCycles) => {},
        _ => panic!(),
    }
}

#[test]
fn test_unused() {
    let merger = GrammarMerger::new()
                 .merge("src/tests/grammars/unused.json").unwrap();
    
    let grammar = ContextFreeGrammar::from_json(merger.grammar()).unwrap();
    println!("{}", grammar);
}

#[test]
fn test_gnf() {
    let merger = GrammarMerger::new()
                 .merge("src/tests/grammars/gnf.json").unwrap();
    
    let mut grammar = ContextFreeGrammar::from_json(merger.grammar()).unwrap();
    grammar.convert_to_gnf();
    //assert!(grammar.is_gnf());

    /*let old_len = grammar.rules().len();
    grammar.convert_to_gnf();
    assert_eq!(old_len, grammar.rules().len());
    */
    println!("{}", grammar);
}

#[test]
fn test_merger() {
    match GrammarMerger::new().merge("src/tests/grammars/part_1.json").unwrap().merge("src/tests/grammars/part_3.json") {
        Err(GrammarError::MergeConflict(_)) => {},
        _ => unreachable!(),
    }
    
    let merger = GrammarMerger::new().merge("src/tests/grammars/part_1.json").unwrap().merge("src/tests/grammars/part_2.json").unwrap();
    println!("{}", ContextFreeGrammar::from_json(merger.grammar()).unwrap());
}

//TODO: extra tests for eliminating left recursion
