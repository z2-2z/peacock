use peacock_fuzz::{
    grammar::ContextFreeGrammar,
    backends::C::CGenerator,
};
use cc;

const GRAMMAR_FILE: &str = "php.json";
const GENERATOR_FILE: &str = "generator.c";

fn main() {
    let cfg = ContextFreeGrammar::builder()
        .gramatron_grammar(GRAMMAR_FILE).unwrap()
        .entrypoint("PROGRAM")
        .build().unwrap();
    
    CGenerator::new().generate(GENERATOR_FILE, cfg);
    
    cc::Build::new()
        .file(GENERATOR_FILE)
        .flag("-O3")
        .flag("-flto")
        .compile("generator");
    
    println!("cargo:rerun-if-changed={}", GRAMMAR_FILE);
}
