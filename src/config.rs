use crate::openapi::{Parameter, ParameterLocation, header_security_schemes, spec_parameters};
use crate::spec::extract_default_server;

pub fn generate_config(spec: &serde_json::Value) -> String {
    let mut variables = std::collections::BTreeSet::new();

    let parameters = spec_parameters(spec, ParameterLocation::Header);
    collect_header_parameters(&parameters, &mut variables);
    collect_header_security_schemes(spec, &mut variables);

    let mut lines = vec!["#!/bin/bash".to_string()];
    lines.push(format!("BASE_URL=\"{}\"", extract_default_server(spec)));
    lines.extend(variables.into_iter().map(|name| format!("{name}=\"\"")));
    lines.push(String::new());
    lines.join("\n")
}

pub fn bash_variable_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect()
}

fn collect_header_parameters(
    parameters: &[Parameter],
    variables: &mut std::collections::BTreeSet<String>,
) {
    for parameter in parameters {
        variables.insert(bash_variable_name(&parameter.name));
    }
}

fn collect_header_security_schemes(
    spec: &serde_json::Value,
    variables: &mut std::collections::BTreeSet<String>,
) {
    for scheme in header_security_schemes(spec) {
        variables.insert(bash_variable_name(&scheme.name));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn given_name_with_hyphens_when_normalizing_then_uses_uppercase_underscores() {
        assert_eq!(
            bash_variable_name("X-DB-Correlation-Id"),
            "X_DB_CORRELATION_ID"
        );
    }

    #[test]
    fn given_camel_case_name_when_normalizing_then_uppercases_without_extra_underscores() {
        assert_eq!(bash_variable_name("accountFilterId"), "ACCOUNTFILTERID");
    }

    #[test]
    fn given_server_when_generating_config_then_contains_base_url() {
        let spec = serde_json::json!({
            "servers": [
                { "url": "https://api.example.com" }
            ],
            "paths": {}
        });

        let config = generate_config(&spec);

        assert_eq!(
            config,
            r#"#!/bin/bash
BASE_URL="https://api.example.com"
"#
        );
    }

    #[test]
    fn given_operation_header_parameters_when_generating_config_then_contains_empty_variables() {
        let spec = serde_json::json!({
            "paths": {
                "/v1/accounts": {
                    "get": {
                        "parameters": [
                            { "name": "Accept-Language", "in": "header", "required": true},
                            { "name": "X-DB-Correlation-Id", "in": "header", "required": true},

                        ]
                    }
                }
            }
        });

        let config = generate_config(&spec);

        assert_eq!(
            config,
            r#"#!/bin/bash
BASE_URL="https://localhost"
ACCEPT_LANGUAGE=""
X_DB_CORRELATION_ID=""
"#
        );
    }
    #[test]
    fn given_header_api_key_security_scheme_when_generating_config_then_contains_variable() {
        let spec = serde_json::json!({
            "components": {
                "securitySchemes": {
                    "clientIdHeader": {
                        "type": "apiKey",
                        "name": "X-IBM-Client-Id",
                        "in": "header"
                    },
                    "ignoredQueryKey": {
                        "type": "apiKey",
                        "name": "api_key",
                        "in": "query"
                    }
                }
            },
            "paths": {}
        });

        let config = generate_config(&spec);

        assert_eq!(
            config,
            r#"#!/bin/bash
BASE_URL="https://localhost"
X_IBM_CLIENT_ID=""
"#
        );
    }
}
