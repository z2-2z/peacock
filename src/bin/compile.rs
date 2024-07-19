use clap::Parser;
use peacock_fuzz::{
    backends::C::CGenerator,
    grammar::ContextFreeGrammar,
};

pub mod fuzz;
use fuzz::GrammarFormat;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, value_name = "GRAMMAR")]
    grammar: String,

    #[arg(long)]
    output: String,

    #[arg(long, default_value_t = GrammarFormat::Peacock)]
    format: GrammarFormat,

    #[arg(short, long)]
    entrypoint: Option<String>,
}

fn main() {
    let args = Args::parse();

    let mut cfg = ContextFreeGrammar::builder();

    match args.format {
        GrammarFormat::Peacock => cfg = cfg.peacock_grammar(&args.grammar).unwrap(),
        GrammarFormat::Gramatron => cfg = cfg.gramatron_grammar(&args.grammar).unwrap(),
    }

    if let Some(entrypoint) = args.entrypoint {
        cfg = cfg.entrypoint(entrypoint);
    }

    let cfg = cfg.build().unwrap();

    CGenerator::new().generate(&args.output, &cfg);
}
