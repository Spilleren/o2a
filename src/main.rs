//! o2a - `OpenApi` to shell script generator
mod config;
mod generator;
mod openapi;
mod path;
mod script;
mod spec;

use std::env;
use std::fs;

use generator::generate_files;
use spec::parse_spec;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: o2a <openapi_file>");
        std::process::exit(1);
    }

    let filename = &args[1];

    let contents = fs::read_to_string(filename)?;
    let spec = parse_spec(&contents)?;

    generate_files(&spec, std::path::Path::new("generated"))?;

    Ok(())
}
