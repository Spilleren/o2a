use crate::config::generate_config;
use crate::path::{path_to_folders, relative_config_path};
use crate::script::generate_script;
use crate::spec::Spec;

use std::fs;
use std::os::unix::fs::PermissionsExt;

pub fn generate_files(spec: &Spec, output_dir: &std::path::Path) -> std::io::Result<()> {
    let paths = &spec.paths;
    let security_headers = spec.header_security_schemes();

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
        PathItem, Server, parse_spec,
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
    fn given_spec_with_ref_parameters_and_security_schemes_when_generating_then_output_is_resolved() {
        let json = r##"{
            "servers": [{"url": "https://api.example.com"}],
            "paths": {
                "/v1/items": {
                    "get": {
                        "parameters": [
                            {"$ref": "#/components/parameters/AcceptLang"},
                            {"name": "limit", "in": "query", "required": false}
                        ]
                    }
                }
            },
            "components": {
                "parameters": {
                    "AcceptLang": { "name": "Accept-Language", "in": "header", "required": true }
                },
                "securitySchemes": {
                    "apiKey": { "type": "apiKey", "name": "X-API-Key", "in": "header" }
                }
            }
        }"##;

        let spec = parse_spec(json).unwrap();
        let output_dir = test_output_dir("ref-golden");

        generate_files(&spec, &output_dir).unwrap();

        let config = std::fs::read_to_string(output_dir.join(".config.sh")).unwrap();
        // Security scheme from resolved ref should appear
        assert!(config.contains("X_API_KEY"), "expected X_API_KEY in config, got:\n{config}");
        // Header parameter from resolved ref should appear
        assert!(config.contains("ACCEPT_LANGUAGE"), "expected ACCEPT_LANGUAGE in config, got:\n{config}");

        let script = std::fs::read_to_string(output_dir.join("v1/items/get")).unwrap();
        // Script should contain the resolved Accept-Language header and X-API-Key security header
        assert_eq!(
            script,
            r#"#!/bin/bash
source "$(dirname "$0")/../../.config.sh"

LIMIT=""

args=(
  --request GET
  --url "$BASE_URL/v1/items"
  --header "Accept-Language: $ACCEPT_LANGUAGE"
  --header "X-API-Key: $X_API_KEY"
)

[[ -n "$LIMIT" ]] && args+=(--url-query "limit=$LIMIT")

curl "${args[@]}""#
        );

        std::fs::remove_dir_all(output_dir).unwrap();
    }
}
