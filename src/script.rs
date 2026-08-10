use crate::config::bash_variable_name;
use crate::openapi::{Parameter, ParameterLocation, operation_parameters};

pub fn generate_script(
    base_url: &str,
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

    let required_query_parameters = query_parameters
        .iter()
        .filter(|p| p.required)
        .cloned()
        .collect::<Vec<_>>();

    let optional_query_parameters = query_parameters
        .iter()
        .filter(|p| !p.required)
        .cloned()
        .collect::<Vec<_>>();

    let path = replace_path_params(path);
    let mut url = format!("{base_url}{path}");

    let query = build_query_string(&required_query_parameters, &optional_query_parameters);

    if !query.is_empty() {
        url.push_str(&query);
    }

    let mut flags: Vec<String> = Vec::new();
    flags.push(format!("  --url \"{url}\""));
    flags.extend(headers.iter().map(|h| format!("  {h}")));

    let variables = parameters_to_variables(
        &path_parameters
            .into_iter()
            .chain(required_query_parameters)
            .chain(optional_query_parameters)
            .collect::<Vec<_>>(),
    );

    let variables_block = if variables.is_empty() {
        String::new()
    } else {
        format!("\n{}\n", variables.join("\n"))
    };
    let curl = format!("curl --request {uc_method} \\\n{}", flags.join(" \\\n"));

    format!(
        r#"#!/bin/bash
source "$(dirname "$0")/{config_path}"
{variables_block}
{curl}"#
    )
}

fn build_query_string(required: &[Parameter], optional: &[Parameter]) -> String {
    let mut query = String::new();

    if required.is_empty() && optional.is_empty() {
        query
    } else {
        for (i, p) in required.iter().enumerate() {
            if i == 0 {
                query.push('?')
            }
            if i > 0 {
                query.push('&');
            }
            let name = p.name.as_str();
            if name.is_empty() {
                continue;
            }
            query.push_str(&format!("{}=${}", name, bash_variable_name(name)));
        }

        for (i, p) in optional.iter().enumerate() {
            let prefix = if query.is_empty() && i == 0 { "?" } else { "&" };
            let name = p.name.as_str();
            if name.is_empty() {
                continue;
            }
            let uc_name = bash_variable_name(name);
            query.push_str(&format!("${{{uc_name}:+{prefix}{name}=${uc_name}}}"));
        }
        query
    }
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
        let script = generate_script(
            "https://api.example.com",
            "/users",
            "get",
            &operation,
            "../config.sh",
            &[],
        );

        assert_eq!(
            script,
            r#"#!/bin/bash
source "$(dirname "$0")/../config.sh"

curl --request GET \
  --url "https://api.example.com/users""#
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

        let script = generate_script(
            "https://api.example.com",
            "/users",
            "get",
            &operation,
            "../../config.sh",
            &[],
        );

        assert_eq!(
            script,
            r#"#!/bin/bash
source "$(dirname "$0")/../../config.sh"

curl --request GET \
  --url "https://api.example.com/users" \
  --header "X-DB-Correlation-Id: $X_DB_CORRELATION_ID""#
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

        let script = generate_script(
            "https://api.example.com",
            "/users",
            "get",
            &operation,
            "../../config.sh",
            &[],
        );

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
                    "name": "account-filter-id",
                    "in": "path",
                    "required": true
                }
            ]
        });
        let script = generate_script(
            "https://api.example.com",
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

curl --request GET \
  --url "https://api.example.com/users/$ACCOUNT_FILTER_ID""#
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
        let script = generate_script(
            "https://api.example.com",
            "/users",
            "get",
            &operation,
            "../../config.sh",
            &[],
        );

        assert_eq!(
            script,
            r#"#!/bin/bash
source "$(dirname "$0")/../../config.sh"

STATUS=""
LIMIT=""

curl --request GET \
  --url "https://api.example.com/users?status=$STATUS&limit=$LIMIT""#
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
        let script = generate_script(
            "https://api.example.com",
            "/users",
            "get",
            &operation,
            "../../config.sh",
            &[],
        );

        assert_eq!(
            script,
            r#"#!/bin/bash
source "$(dirname "$0")/../../config.sh"

STATUS=""
LIMIT=""

curl --request GET \
  --url "https://api.example.com/users?status=$STATUS${LIMIT:+&limit=$LIMIT}""#
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
        let script = generate_script(
            "https://api.example.com",
            "/users",
            "get",
            &operation,
            "../../config.sh",
            &[],
        );

        assert_eq!(
            script,
            r#"#!/bin/bash
source "$(dirname "$0")/../../config.sh"

LIMIT=""

curl --request GET \
  --url "https://api.example.com/users${LIMIT:+?limit=$LIMIT}""#
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
        let script = generate_script(
            "https://api.example.com",
            "/users",
            "get",
            &operation,
            "../../config.sh",
            &[],
        );

        assert_eq!(
            script,
            r#"#!/bin/bash
source "$(dirname "$0")/../../config.sh"

LIMIT=""

curl --request GET \
  --url "https://api.example.com/users${LIMIT:+?limit=$LIMIT}""#
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

        let script = generate_script(
            "https://api.example.com",
            "/users/{id}",
            "get",
            &operation,
            "../../config.sh",
            &[],
        );

        assert_eq!(
            script,
            r#"#!/bin/bash
source "$(dirname "$0")/../../config.sh"

ID=""
STATUS=""

curl --request GET \
  --url "https://api.example.com/users/$ID?status=$STATUS""#
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
            "https://api.example.com",
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

curl --request GET \
  --url "https://api.example.com/users" \
  --header "X-IBM-Client-Id: $X_IBM_CLIENT_ID""#
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
