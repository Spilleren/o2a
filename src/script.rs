pub fn generate_script(
    base_url: &str,
    path: &str,
    method: &str,
    operation: &serde_json::Value,
) -> String {
    let uc_method = method.to_uppercase();
    let header_parameters = extract_parameters(operation, "header");
    let path_parameters = extract_parameters(operation, "path");
    let query_parameters = extract_parameters(operation, "query");

    let headers = parameters_to_flags(&header_parameters, |name, var| {
        format!("--header \"{name}: ${var}\"")
    });

    let required_query_parameters = query_parameters
        .iter()
        .filter(|p| p["required"] == true)
        .cloned()
        .collect::<Vec<_>>();

    let optional_query_parameters = query_parameters
        .iter()
        .filter(|p| p["required"] == false)
        .cloned()
        .collect::<Vec<_>>();

    let required_query = parameters_to_flags(&required_query_parameters, |name, var| {
        format!("--data-urlencode \"{name}=${var}\"")
    });

    let optional_query = parameters_to_flags(&optional_query_parameters, |name, var| {
        format!("# --data-urlencode \"{name}=${var}\"")
    });

    let path = replace_path_params(path);
    let variables = parameters_to_variables(
        &path_parameters
            .into_iter()
            .chain(required_query_parameters)
            .chain(optional_query_parameters)
            .collect::<Vec<_>>(),
    );

    let mut flags: Vec<String> = Vec::new();
    flags.push(format!("  --url \"{base_url}{path}\""));
    flags.extend(headers.iter().map(|h| format!("  {h}")));
    flags.extend(required_query.iter().map(|q| format!("  {q}")));
    flags.extend(optional_query.iter().map(|q| format!("  {q}")));

    let variables_block = if variables.is_empty() {
        String::new()
    } else {
        format!("\n{}\n", variables.join("\n"))
    };
    let curl = format!("curl --request {uc_method} \\\n{}", flags.join(" \\\n"));

    format!(
        r#"#!/bin/bash
source "$(dirname "$0")/../../config.sh"
{variables_block}
{curl}"#
    )
}

fn parameters_to_flags(
    params: &[serde_json::Value],
    format_fn: impl Fn(&str, &str) -> String,
) -> Vec<String> {
    params
        .iter()
        .map(|p| {
            let name = p["name"].as_str().unwrap_or("");
            let var = name.to_uppercase();
            format_fn(name, &var)
        })
        .collect()
}

fn parameters_to_variables(params: &[serde_json::Value]) -> Vec<String> {
    params
        .iter()
        .map(|p| {
            let name = p["name"].as_str().unwrap_or("").to_uppercase();
            if p["required"] == true {
                format!("{name}=\"\"")
            } else {
                format!("# {name}=\"\"")
            }
        })
        .collect()
}

fn replace_path_params(path: &str) -> String {
    path.split('/')
        .map(|segment| {
            if segment.starts_with('{') && segment.ends_with('}') {
                format!("${}", &segment[1..segment.len() - 1].to_uppercase())
            } else {
                segment.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}
fn extract_parameters(operation: &serde_json::Value, in_value: &str) -> Vec<serde_json::Value> {
    operation["parameters"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|p| p["in"] == in_value)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use serde_json::json;

    #[test]
    fn given_get_operation_with_no_parameters_when_generating_script_then_produces_correct_curl_command()
     {
        let operation = json!({});

        let script = generate_script("https://api.example.com", "/users", "get", &operation);

        assert_eq!(
            script,
            r#"#!/bin/bash
source "$(dirname "$0")/../../config.sh"

curl --request GET \
  --url "https://api.example.com/users""#
        );
    }

    #[test]
    fn given_operation_with_header_parameter_when_generating_script_then_contains_header() {
        let operation = json!({
            "parameters": [
                {
                    "name": "Authorization",
                    "in": "header",
                    "required": true
                }
            ]
        });

        let script = generate_script("https://api.example.com", "/users", "get", &operation);

        assert_eq!(
            script,
            r#"#!/bin/bash
source "$(dirname "$0")/../../config.sh"

curl --request GET \
  --url "https://api.example.com/users" \
  --header "Authorization: $AUTHORIZATION""#
        );
    }

    #[test]
    fn given_operation_with_multiple_header_parameter_when_generating_script_then_contains_header()
    {
        let operation = json!({
            "parameters": [
                {
                    "name": "Authorization",
                    "in": "header",
                    "required": true
                },
                {
                    "name": "AcceptLanguage",
                    "in": "header",
                    "required": true
                }
            ]
        });

        let script = generate_script("https://api.example.com", "/users", "get", &operation);

        assert_eq!(
            script,
            r#"#!/bin/bash
source "$(dirname "$0")/../../config.sh"

curl --request GET \
  --url "https://api.example.com/users" \
  --header "Authorization: $AUTHORIZATION" \
  --header "AcceptLanguage: $ACCEPTLANGUAGE""#
        );
    }

    #[test]
    fn given_operation_with_path_parameter_when_generating_script_then_url_contains_shell_variable()
    {
        let operation = json!({
            "parameters": [
                {
                    "name": "id",
                    "in": "path",
                    "required": true
                }
            ]
        });
        let script = generate_script("https://api.example.com", "/users/{id}", "get", &operation);

        assert_eq!(
            script,
            r#"#!/bin/bash
source "$(dirname "$0")/../../config.sh"

ID=""

curl --request GET \
  --url "https://api.example.com/users/$ID""#
        );
    }

    #[test]
    fn given_operation_with_query_parameter_when_generating_script_then_contains_query_parameters()
    {
        let operation = json!({
            "parameters": [
                {
                    "name": "status",
                    "in": "query",
                    "required": true
                }
            ]
        });
        let script = generate_script("https://api.example.com", "/users", "get", &operation);

        assert_eq!(
            script,
            r#"#!/bin/bash
source "$(dirname "$0")/../../config.sh"

STATUS=""

curl --request GET \
  --url "https://api.example.com/users" \
  --data-urlencode "status=$STATUS""#
        );
    }

    #[test]
    fn given_operation_with_optional_query_parameters_when_generating_script_then_parameter_is_commented_out()
     {
        let operation = json!({
            "parameters": [
                {
                    "name": "status",
                    "in": "query",
                    "required": true
                },
                {
                    "name": "limit",
                    "in": "query",
                    "required": false
                },
            ]
        });
        let script = generate_script("https://api.example.com", "/users", "get", &operation);

        assert_eq!(
            script,
            r#"#!/bin/bash
source "$(dirname "$0")/../../config.sh"

STATUS=""
# LIMIT=""

curl --request GET \
  --url "https://api.example.com/users" \
  --data-urlencode "status=$STATUS" \
  # --data-urlencode "limit=$LIMIT""#
        );
    }

    #[test]
    fn given_operation_with_query_and_path_parameters_when_generating_script_then_contains_query_parameters()
     {
        let operation = json!({
            "parameters": [
                {
                    "name": "status",
                    "in": "query",
                    "required": true
                },
                {
                    "name": "id",
                    "in": "path",
                    "required": true
                }
            ]
        });
        let script = generate_script("https://api.example.com", "/users/{id}", "get", &operation);

        assert_eq!(
            script,
            r#"#!/bin/bash
source "$(dirname "$0")/../../config.sh"

ID=""
STATUS=""

curl --request GET \
  --url "https://api.example.com/users/$ID" \
  --data-urlencode "status=$STATUS""#
        );
    }
}
