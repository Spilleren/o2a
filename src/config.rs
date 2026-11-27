use crate::openapi::spec_parameters;
use crate::spec::{Parameter, ParameterLocation, Spec, bash_variable_name, prefer_https};

pub fn generate_config(spec: &Spec) -> String {
    let mut variables = std::collections::BTreeSet::new();

    let parameters = spec_parameters(spec, ParameterLocation::Header);
    collect_header_parameters(&parameters, &mut variables);
    collect_header_security_schemes(spec, &mut variables);

    let mut lines = vec!["#!/bin/bash".to_string()];
    let default_server = spec.servers.first().map(|server| server.url.as_str()).unwrap_or("http://localhost");
    lines.push(format!("BASE_URL=\"{}\"", prefer_https(default_server)));
    lines.extend(variables.into_iter().map(|name| format!("{name}=\"\"")));
    lines.push(String::new());
    lines.join("\n")
}

fn collect_header_parameters(parameters: &[Parameter], variables: &mut std::collections::BTreeSet<String>) {
    for parameter in parameters {
        variables.insert(bash_variable_name(&parameter.name));
    }
}

fn collect_header_security_schemes(spec: &Spec, variables: &mut std::collections::BTreeSet<String>) {
    for scheme in spec.header_security_schemes() {
        variables.insert(bash_variable_name(&scheme.name));
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::spec::{
        Components, Operation,
        ParameterLocation::{Header, Query},
        PathItem, SecurityScheme, Server,
    };

    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn given_server_when_generating_config_then_contains_base_url() {
        let spec = Spec {
            servers: vec![Server { url: "https://api.example.com".to_owned() }],
            paths: BTreeMap::new(),
            components: Components::default(),
        };

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
        let spec = Spec {
            servers: vec![],
            paths: BTreeMap::from([(
                "/v1/accounts".to_owned(),
                PathItem {
                    get: Some(Operation {
                        parameters: vec![
                            Parameter { name: "Accept-Language".to_owned(), location: Header, required: false },
                            Parameter { name: "X-DB-Correlation-Id".to_owned(), location: Header, required: true },
                        ],
                    }),
                    ..Default::default()
                },
            )]),
            components: Components::default(),
        };
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
        let spec = Spec {
            servers: vec![],
            paths: BTreeMap::new(),
            components: Components {
                security_schemes: BTreeMap::from([
                    (
                        "clientIdHeader".to_owned(),
                        SecurityScheme::ApiKey { name: "X-IBM-Client-Id".to_owned(), location: Header },
                    ),
                    (
                        "ignoredQueryKey".to_owned(),
                        SecurityScheme::ApiKey { name: "api_key".to_owned(), location: Query },
                    ),
                ]),
            },
        };

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
