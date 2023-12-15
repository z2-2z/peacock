use clap::Parser;

use peacock_fuzz::{
    grammar::ContextFreeGrammar,
    backends::json::JsonGenerator,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, required = true)]
    output: String,
    
    #[arg(long, required = false)]
    peacock_grammar: Vec<String>,
    
    #[arg(long, required = false)]
    gramatron_grammar: Vec<String>,
    
    #[arg(short, long)]
    entrypoint: Option<String>,
    
    #[arg(long, default_value_t = false)]
    optimize: bool,
}

fn main() {
    let args = Args::parse();
    
    if args.peacock_grammar.is_empty() && args.gramatron_grammar.is_empty() {
        panic!("You need to supply at least one grammar");
    }
    
    let mut builder = ContextFreeGrammar::builder();
    
    for path in &args.peacock_grammar {
        builder = builder.peacock_grammar(path).unwrap();
    }
    
    for path in &args.gramatron_grammar {
        builder = builder.gramatron_grammar(path).unwrap();
    }
    
    builder = builder.optimize(args.optimize);
    
    if let Some(entrypoint) = args.entrypoint {
        builder = builder.entrypoint(entrypoint);
    }
    
    let cfg = builder.build().unwrap();
    JsonGenerator::new().generate(args.output, cfg);
}
