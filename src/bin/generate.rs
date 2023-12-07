use clap::Parser;
use peacock_fuzz::{
    grammar::ContextFreeGrammar,
    backends::C::CGenerator,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
enum GrammarFormat {
    Peacock,
    Gramatron,
}

impl std::fmt::Display for GrammarFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GrammarFormat::Peacock => write!(f, "peacock"),
            GrammarFormat::Gramatron => write!(f, "gramatron"),
        }
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, value_name = "GRAMMAR")]
    grammar: String,
    
    #[arg(long)]
    output: String,
    
    #[arg(long, default_value_t = GrammarFormat::Peacock)]
    format: GrammarFormat,
}

fn main() {
    let args = Args::parse();
    
    let mut cfg = ContextFreeGrammar::builder();
        
    match args.format {
        GrammarFormat::Peacock => cfg = cfg.peacock_grammar(&args.grammar).unwrap(),
        GrammarFormat::Gramatron => cfg = cfg.gramatron_grammar(&args.grammar).unwrap(),
    }
    
    let cfg = cfg.build().unwrap();
    
    CGenerator::new(&args.output).generate(cfg);
}
