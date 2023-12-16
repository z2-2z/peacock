use clap::Parser;
use std::io::Write;
use libafl::prelude::{
    Input, HasTargetBytes,
};
use libafl_bolts::prelude::AsSlice;
use peacock_fuzz::components::{
    load_generator,
    PeacockInput,
};   

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    generator: String,
    
    file: String,
}

fn main() {
    let args = Args::parse();    
    load_generator(&args.generator);
    let input = PeacockInput::from_file(&args.file).expect("Could not load specified input file");
    let input = input.target_bytes();
    std::io::stdout().write_all(input.as_slice()).expect("Could not write to stdout");
}
