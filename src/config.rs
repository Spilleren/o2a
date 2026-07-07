pub fn generate_config(spec: &serde_json::Value) -> String {
    let mut variables = std::collections::BTreeSet::new();

    if let Some(paths) = spec["paths"].as_object() {
        for path_item in paths.values() {
            if let Some(methods) = path_item.as_object() {
                for operation in methods.values() {
                    collect_header_parameters(operation, &mut variables);
                }
            }
        }
    }
    let mut lines = vec!["#!/bin/bash".to_string()];
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
    operation: &serde_json::Value,
    variables: &mut std::collections::BTreeSet<String>,
) {
    let Some(parameters) = operation["parameters"].as_array() else {
        return;
    };

    for parameter in parameters {
        if parameter["in"] == "header" {
            if let Some(name) = parameter["name"].as_str() {
                variables.insert(bash_variable_name(name));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
ACCEPT_LANGUAGE=""
X_DB_CORRELATION_ID=""
"#
        );
    }
}
