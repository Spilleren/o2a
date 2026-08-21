use crate::config::bash_variable_name;
use crate::openapi::{Parameter, ParameterLocation, operation_parameters};

pub fn generate_script(
    path: &str,
    method: &str,
    operation: &serde_json::Value,
    config_path: &str,
    security_headers: &[Parameter],
) -> String {
    let uc_method = method.to_uppercase();
    let header_parameters = operation_parameters(operation, ParameterLocation::Header);
    let path_parameters = operation_parameters(operation, ParameterLocation::Path);
    let query_parameters = operation_parameters(operation, ParameterLocation::Query);

    let mut headers = parameters_to_flags(&header_parameters, |name, var| {
        format!("--header \"{name}: ${var}\"")
    });

    headers.extend(parameters_to_flags(security_headers, |name, var| {
        format!("--header \"{name}: ${var}\"")
    }));

    let path = replace_path_params(path);
    let url = format!("$BASE_URL{path}");

    let mut args: Vec<String> = vec![
        format!("  --request {uc_method}"),
        format!("  --url \"{url}\""),
    ];
    args.extend(headers.iter().map(|h| format!("  {h}")));

    let query_lines = parameters_to_query_lines(&query_parameters);
    let guards = parameters_to_guards(&query_parameters);

    let script_parameters: Vec<Parameter> = path_parameters
        .into_iter()
        .chain(query_parameters)
        .collect();

    let variables = parameters_to_variables(&script_parameters);

    let mut blocks: Vec<String> = Vec::new();
    if !variables.is_empty() {
        blocks.push(variables.join("\n"));
    }
    if !guards.is_empty() {
        blocks.push(guards.join("\n"));
    }
    blocks.push(format!("args=(\n{}\n)", args.join("\n")));
    if !query_lines.is_empty() {
        blocks.push(query_lines.join("\n"));
    }
    blocks.push("curl \"${args[@]}\"".to_string());

    let body = blocks.join("\n\n");

    format!(
        r#"#!/bin/bash
source "$(dirname "$0")/{config_path}"

{body}"#
    )
}

fn parameters_to_query_lines(params: &[Parameter]) -> Vec<String> {
    params
        .iter()
        .map(|p| {
            let name = p.name.as_str();
            let var = bash_variable_name(name);
            if p.required {
                format!("args+=(--url-query \"{name}=${var}\")")
            } else {
                format!("[[ -n \"${var}\" ]] && args+=(--url-query \"{name}=${var}\")")
            }
        })
        .collect()
}

fn parameters_to_guards(params: &[Parameter]) -> Vec<String> {
    params
        .iter()
        .filter(|p| p.required)
        .map(|p| {
            let var = bash_variable_name(p.name.as_str());
            format!(": \"${{{var}:?{var} is required}}\"")
        })
        .collect()
}

fn parameters_to_flags(
    params: &[Parameter],
    format_fn: impl Fn(&str, &str) -> String,
) -> Vec<String> {
    params
        .iter()
        .map(|p| {
            let name = p.name.as_str();
            let var = bash_variable_name(name);
            format_fn(name, &var)
        })
        .collect()
}

fn parameters_to_variables(params: &[Parameter]) -> Vec<String> {
    params
        .iter()
        .map(|p| format!("{}=\"\"", bash_variable_name(p.name.as_str())))
        .collect()
}

fn replace_path_params(path: &str) -> String {
    path.split('/')
        .map(|segment| {
            if segment.starts_with('{') && segment.ends_with('}') {
                let name = &segment[1..segment.len() - 1];
                format!("${}", bash_variable_name(name))
            } else {
                segment.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("/")
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
        let script = generate_script("/users", "get", &operation, "../config.sh", &[]);

        assert_eq!(
            script,
            r#"#!/bin/bash
source "$(dirname "$0")/../config.sh"

args=(
  --request GET
  --url "$BASE_URL/users"
)

curl "${args[@]}""#
        );
    }

    #[test]
    fn given_operation_with_header_parameter_when_generating_script_then_contains_header() {
        let operation = json!({
            "parameters": [
                {
                    "name": "X-DB-Correlation-Id",
                    "in": "header",
                    "required": true
                }
            ]
        });
        let script = generate_script("/users", "get", &operation, "../../config.sh", &[]);

        assert_eq!(
            script,
            r#"#!/bin/bash
source "$(dirname "$0")/../../config.sh"

args=(
  --request GET
  --url "$BASE_URL/users"
  --header "X-DB-Correlation-Id: $X_DB_CORRELATION_ID"
)

curl "${args[@]}""#
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
        let script = generate_script("/users", "get", &operation, "../../config.sh", &[]);

        assert_eq!(
            script,
            r#"#!/bin/bash
source "$(dirname "$0")/../../config.sh"

args=(
  --request GET
  --url "$BASE_URL/users"
  --header "Authorization: $AUTHORIZATION"
  --header "AcceptLanguage: $ACCEPTLANGUAGE"
)

curl "${args[@]}""#
        );
    }

    #[test]
    fn given_operation_with_path_parameter_when_generating_script_then_url_contains_shell_variable()
    {
        let operation = json!({
            "parameters": [
                {
                    "name": "account-filter-id",
                    "in": "path",
                    "required": true
                }
            ]
        });
        let script = generate_script(
            "/users/{account-filter-id}",
            "get",
            &operation,
            "../../config.sh",
            &[],
        );

        assert_eq!(
            script,
            r#"#!/bin/bash
source "$(dirname "$0")/../../config.sh"

ACCOUNT_FILTER_ID=""

args=(
  --request GET
  --url "$BASE_URL/users/$ACCOUNT_FILTER_ID"
)

curl "${args[@]}""#
        );
    }

    #[test]
    fn given_operation_with_query_parameters_when_generating_script_then_contains_query_parameters()
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
                    "required": true
                }

            ]
        });
        let script = generate_script("/users", "get", &operation, "../../config.sh", &[]);

        assert_eq!(
            script,
            r#"#!/bin/bash
source "$(dirname "$0")/../../config.sh"

STATUS=""
LIMIT=""

: "${STATUS:?STATUS is required}"
: "${LIMIT:?LIMIT is required}"

args=(
  --request GET
  --url "$BASE_URL/users"
)

args+=(--url-query "status=$STATUS")
args+=(--url-query "limit=$LIMIT")

curl "${args[@]}""#
        );
    }

    #[test]
    fn given_operation_with_required_and_optional_query_parameters_when_generating_script_then_query_parameters()
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
        let script = generate_script("/users", "get", &operation, "../../config.sh", &[]);

        assert_eq!(
            script,
            r#"#!/bin/bash
source "$(dirname "$0")/../../config.sh"

STATUS=""
LIMIT=""

: "${STATUS:?STATUS is required}"

args=(
  --request GET
  --url "$BASE_URL/users"
)

args+=(--url-query "status=$STATUS")
[[ -n "$LIMIT" ]] && args+=(--url-query "limit=$LIMIT")

curl "${args[@]}""#
        );
    }

    #[test]
    fn given_operation_with_optional_query_parameters_when_generating_script_then_query_parameters()
    {
        let operation = json!({
            "parameters": [
                {
                    "name": "limit",
                    "in": "query",
                    "required": false
                }
            ]
        });
        let script = generate_script("/users", "get", &operation, "../../config.sh", &[]);

        assert_eq!(
            script,
            r#"#!/bin/bash
source "$(dirname "$0")/../../config.sh"

LIMIT=""

args=(
  --request GET
  --url "$BASE_URL/users"
)

[[ -n "$LIMIT" ]] && args+=(--url-query "limit=$LIMIT")

curl "${args[@]}""#
        );
    }

    #[test]
    fn given_operation_with_optional_query_parameters_with_no_required_field_when_generating_script_then_query_parameters()
     {
        let operation = json!({
            "parameters": [
                {
                    "name": "limit",
                    "in": "query"
                }
            ]
        });
        let script = generate_script("/users", "get", &operation, "../../config.sh", &[]);

        assert_eq!(
            script,
            r#"#!/bin/bash
source "$(dirname "$0")/../../config.sh"

LIMIT=""

args=(
  --request GET
  --url "$BASE_URL/users"
)

[[ -n "$LIMIT" ]] && args+=(--url-query "limit=$LIMIT")

curl "${args[@]}""#
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
        let script = generate_script("/users/{id}", "get", &operation, "../../config.sh", &[]);

        assert_eq!(
            script,
            r#"#!/bin/bash
source "$(dirname "$0")/../../config.sh"

ID=""
STATUS=""

: "${STATUS:?STATUS is required}"

args=(
  --request GET
  --url "$BASE_URL/users/$ID"
)

args+=(--url-query "status=$STATUS")

curl "${args[@]}""#
        );
    }

    #[test]
    fn given_security_scheme_header_when_generating_script_then_contains_security_header() {
        let operation = json!({});

        let security_headers = vec![Parameter {
            name: String::from("X-IBM-Client-Id"),
            location: ParameterLocation::Header,
            required: true,
        }];
        let script = generate_script(
            "/users",
            "get",
            &operation,
            "../.config.sh",
            &security_headers,
        );

        assert_eq!(
            script,
            r#"#!/bin/bash
source "$(dirname "$0")/../.config.sh"

args=(
  --request GET
  --url "$BASE_URL/users"
  --header "X-IBM-Client-Id: $X_IBM_CLIENT_ID"
)

curl "${args[@]}""#
        );
    }
    //     #[test]
    //     fn given_post_operation_with_no_request_body_when_generation_script_then_procudes_empty_request_body_variable()
    //      {
    //         let operation = json!({
    //             "requestbody": {
    //                 "content": {
    //                     "application/json": {
    //                         "schema": {
    //                             "type": "object"
    //                         }
    //                     }
    //                 }
    //             }
    //         });
    //
    //         let script = generate_script("https://api.example.com", "/items", "post", &operation);
    //
    //         assert_eq!(
    //             script,
    //             r#"#!/bin/bash
    // source "$(dirname "$0")/../../config.sh"
    //
    // REQUEST_BODY="{}"
    //
    // curl --request POST \
    //   --url "https://api.example.com/items" \
    //   --data "$REQUEST_BODY""#
    //         );
    //     }
}
