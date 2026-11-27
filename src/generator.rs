use crate::config::generate_config;
use crate::openapi::header_security_schemes;
use crate::path::{path_to_folders, relative_config_path};
use crate::script::generate_script;
use crate::spec::{Spec, extract_paths};

use std::fs;
use std::os::unix::fs::PermissionsExt;

pub fn generate_files(spec: &Spec, output_dir: &std::path::Path) -> std::io::Result<()> {
    let paths = extract_paths(spec);
    let security_headers = header_security_schemes(spec);

    fs::create_dir_all(output_dir)?;
    fs::write(output_dir.join(".config.sh"), generate_config(spec))?;

    for (path, methods) in paths {
        let methods = methods.operations();
        let folder = path_to_folders(path);

        for (method, operation) in methods {
            let script = generate_script(path, method, operation, &relative_config_path(&folder), &security_headers);

            let dir_path = output_dir.join(&folder);
            let file_path = dir_path.join(method.to_lowercase());

            fs::create_dir_all(&dir_path)?;
            fs::write(&file_path, script)?;
            let mut perms = fs::metadata(&file_path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&file_path, fs::Permissions::from_mode(0o755))?;
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use crate::spec::{
        Components, Operation, Parameter,
        ParameterLocation::{Header, Path},
        PathItem, SecurityScheme, Server,
    };

    use super::*;
    use pretty_assertions::assert_eq;
    use std::collections::BTreeMap;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_output_dir(test_name: &str) -> PathBuf {
        let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        std::env::temp_dir().join(format!("o2a-{test_name}-{unique}"))
    }

    fn spec_with_users_get() -> Spec {
        Spec {
            servers: vec![Server { url: "https://api.example.com".to_owned() }],
            paths: BTreeMap::from([(
                "/v1/users".to_owned(),
                PathItem {
                    get: Some(Operation {
                        parameters: vec![Parameter {
                            name: "Accept-Language".to_owned(),
                            location: Header,
                            required: false,
                        }],
                    }),
                    ..Default::default()
                },
            )]),
            components: Components::default(),
        }
    }

    #[test]
    fn given_spec_when_generating_files_then_writes_config_file_to_output_dir() {
        let spec = spec_with_users_get();
        let output_dir = test_output_dir("config-file");

        generate_files(&spec, &output_dir).unwrap();

        let config = std::fs::read_to_string(output_dir.join(".config.sh")).unwrap();
        assert_eq!(
            config,
            r#"#!/bin/bash
BASE_URL="https://api.example.com"
ACCEPT_LANGUAGE=""
"#
        );

        std::fs::remove_dir_all(output_dir).unwrap();
    }

    #[test]
    fn given_spec_when_generating_files_then_writes_endpoint_script_to_output_dir() {
        let spec = spec_with_users_get();
        let output_dir = test_output_dir("endpoint-script");

        generate_files(&spec, &output_dir).unwrap();

        let script = std::fs::read_to_string(output_dir.join("v1/users/get")).unwrap();
        assert_eq!(
            script,
            r#"#!/bin/bash
source "$(dirname "$0")/../../.config.sh"

args=(
  --request GET
  --url "$BASE_URL/v1/users"
  --header "Accept-Language: $ACCEPT_LANGUAGE"
)

curl "${args[@]}""#
        );

        std::fs::remove_dir_all(output_dir).unwrap();
    }

    #[test]
    fn given_nested_path_when_generating_files_then_script_sources_config_from_generated_root() {
        let spec = Spec {
            servers: vec![Server { url: "https://api.example.com".to_owned() }],
            paths: BTreeMap::from([(
                "/v1/users/{user-id}".to_owned(),
                PathItem {
                    get: Some(Operation {
                        parameters: vec![Parameter { name: "user-id".to_owned(), location: Path, required: true }],
                    }),
                    ..Default::default()
                },
            )]),
            components: Components::default(),
        };
        let output_dir = test_output_dir("nested-source");

        generate_files(&spec, &output_dir).unwrap();

        let script = std::fs::read_to_string(output_dir.join("v1/users/_user-id/get")).unwrap();
        assert_eq!(
            script,
            r#"#!/bin/bash
source "$(dirname "$0")/../../../.config.sh"

USER_ID=""

args=(
  --request GET
  --url "$BASE_URL/v1/users/$USER_ID"
)

curl "${args[@]}""#
        );

        std::fs::remove_dir_all(output_dir).unwrap();
    }

    #[test]
    fn given_header_security_scheme_when_generating_files_then_endpoint_script_contains_security_header() {
        let spec = Spec {
            servers: vec![Server { url: "https://api.example.com".to_owned() }],
            paths: BTreeMap::from([(
                "/v1/users".to_owned(),
                PathItem { get: Some(Operation { parameters: vec![] }), ..Default::default() },
            )]),
            components: Components {
                security_schemes: BTreeMap::from([(
                    "clientIdHeader".to_owned(),
                    SecurityScheme::ApiKey { name: "X-IBM-Client-Id".to_owned(), location: Header },
                )]),
            },
        };

        let output_dir = test_output_dir("security-header");

        generate_files(&spec, &output_dir).unwrap();

        let script = std::fs::read_to_string(output_dir.join("v1/users/get")).unwrap();
        assert!(
            script.contains(r#"--header "X-IBM-Client-Id: $X_IBM_CLIENT_ID""#),
            "expected generated script to contain security scheme header, got:\n{script}"
        );

        std::fs::remove_dir_all(output_dir).unwrap();
    }
}
