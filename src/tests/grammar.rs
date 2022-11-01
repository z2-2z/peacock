use crate::grammar::ContextFreeGrammar;

#[test]
fn test_syntax() {
    let grammar = ContextFreeGrammar::from_json("src/tests/grammars/syntax.json").unwrap();
    println!("{}", grammar);
}
