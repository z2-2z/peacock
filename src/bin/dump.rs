use clap::Parser;
use std::path::Path;
use std::io::Write;
use libafl::prelude::{
    Input, HasTargetBytes,
};
use libafl_bolts::prelude::AsSlice;

mod peacock;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short)]
    output: String,
    
    file: String,
}

fn load_generator(output: &str) {
    let path = Path::new(output).join("generator.so");
    peacock::load_generator(path);
}

fn main() {
    let args = Args::parse();
    load_generator(&args.output);
    let input = peacock::PeacockInput::from_file(&args.file).expect("Could not load specified input file");
    let input = input.target_bytes();
    std::io::stdout().write_all(input.as_slice()).expect("Could not write to stdout");
}
