use clap::Parser;
use peacock_fuzz::{
    backends::interpreter::GrammarInterpreter,
    grammar::ContextFreeGrammar,
};
use std::{
    io::{
        stdout,
        BufWriter,
        Write,
    },
    time::SystemTime,
};

pub mod fuzz;
use fuzz::GrammarFormat;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    grammar: String,

    #[arg(long, default_value_t = GrammarFormat::Peacock)]
    format: GrammarFormat,

    #[arg(short, long)]
    entrypoint: Option<String>,

    #[arg(long, default_value_t = false)]
    dont_optimize: bool,

    #[arg(long, short, default_value_t = String::from("1"))]
    count: String,

    #[arg(long, short)]
    seed: Option<String>,
}

fn main() {
    let args = Args::parse();

    let count = args.count.parse::<usize>().unwrap();

    let mut builder = ContextFreeGrammar::builder();

    match args.format {
        GrammarFormat::Peacock => builder = builder.peacock_grammar(args.grammar).unwrap(),
        GrammarFormat::Gramatron => builder = builder.gramatron_grammar(args.grammar).unwrap(),
    }

    if let Some(entrypoint) = args.entrypoint {
        builder = builder.entrypoint(entrypoint);
    }

    builder = builder.optimize(!args.dont_optimize);

    let cfg = builder.build().unwrap();

    let mut stream = BufWriter::new(stdout());
    let mut interpreter = GrammarInterpreter::new(&cfg);

    if let Some(seed) = args.seed {
        let seed = seed.parse::<usize>().unwrap();
        interpreter.seed(seed);
    } else {
        let seed =
            SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() ^ std::process::id() as u64;
        interpreter.seed(seed as usize);
    }

    for _ in 0..count {
        interpreter.interpret(&mut stream).unwrap();
        writeln!(&mut stream).unwrap();
    }

    stream.flush().unwrap();
}
