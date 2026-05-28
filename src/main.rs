mod path;
mod script;
mod spec;

use std::env;
use std::fs;

use spec::{extract_default_server, extract_paths, parse_spec};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: openapi-to-curl <openapi_file>");
        std::process::exit(1);
    }

    let filename = &args[1];

    let contents = fs::read_to_string(filename).expect("Failed to read OpenAPI file");
    let spec = parse_spec(&contents);
    let default_server = extract_default_server(&spec);
    let paths = extract_paths(&spec);

    for (path, methods) in paths {
        let methods = methods.as_object().unwrap();
    }
}
