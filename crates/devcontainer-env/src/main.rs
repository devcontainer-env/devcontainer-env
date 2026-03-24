mod app;

use crate::app::cli::*;
use clap::Parser;

fn main() {
    let program = Program::parse();
    println!("{:?}", program);
}
