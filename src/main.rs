mod path;
mod script;
mod spec;

use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;

use spec::{extract_default_server, extract_paths, parse_spec};

use path::path_to_folders;
use script::generate_script;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: o2a <openapi_file>");
        std::process::exit(1);
    }

    let filename = &args[1];

    let contents = fs::read_to_string(filename).expect("Failed to read OpenAPI file");
    let spec = parse_spec(&contents);
    let default_server = extract_default_server(&spec);
    let paths = extract_paths(&spec);
    for (path, methods) in paths {
        let methods = methods.as_object().unwrap();
        let folder = path_to_folders(path);

        for (method, operation) in methods {
            let script = generate_script(&default_server, path, method, operation);
            let file_path = format!("generated/{}/{}", method, method.to_lowercase());

            fs::create_dir_all(format!("generated/{}", folder)).unwrap();
            fs::write(&file_path, script).unwrap();
            let mut perms = fs::metadata(&file_path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&file_path, fs::Permissions::from_mode(0o755)).unwrap();
        }
    }
}
