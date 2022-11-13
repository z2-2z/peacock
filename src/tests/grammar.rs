use crate::{
    error::Error,
    grammar::{cfg::ContextFreeGrammar, merge::GrammarMerger},
};

#[test]
fn test_syntax() {
    let merger = GrammarMerger::new().merge("src/tests/grammars/syntax.json").unwrap();

    match ContextFreeGrammar::from_dict(merger.dict()) {
        Err(Error::InvalidGrammar(s)) => panic!("{}", s),
        _ => {},
    }
}

#[test]
fn test_cycles() {
    let merger = GrammarMerger::new().merge("src/tests/grammars/cycles.json").unwrap();

    match ContextFreeGrammar::from_dict(merger.dict()) {
        Err(Error::GrammarContainsCycles) => {},
        _ => panic!(),
    }
}

#[test]
fn test_unused() {
    let merger = GrammarMerger::new().merge("src/tests/grammars/unused.json").unwrap();

    let grammar = ContextFreeGrammar::from_dict(merger.dict()).unwrap();

    assert_eq!(grammar.rules().len(), 3);

    println!("{}", grammar);
}

#[test]
fn test_gnf() {
    let merger = GrammarMerger::new().merge("src/tests/grammars/gnf.json").unwrap();

    let mut grammar = ContextFreeGrammar::from_dict(merger.dict()).unwrap();
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
        Err(Error::GrammarMergeConflict(_)) => {},
        _ => unreachable!(),
    }

    let merger = GrammarMerger::new().merge("src/tests/grammars/part_1.json").unwrap().merge("src/tests/grammars/part_2.json").unwrap();
    println!("{}", ContextFreeGrammar::from_dict(merger.dict()).unwrap());
}

//TODO: extra tests for eliminating left recursion
