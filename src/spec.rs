use std::collections::BTreeMap;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Spec {
    #[serde(default)]
    pub servers: Vec<Server>,

    #[serde(default)]
    pub paths: BTreeMap<String, PathItem>,

    #[serde(default)]
    pub components: Components,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Server {
    pub url: String,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct PathItem {
    pub get: Option<Operation>,
    pub put: Option<Operation>,
    pub post: Option<Operation>,
    pub delete: Option<Operation>,
    pub options: Option<Operation>,
    pub head: Option<Operation>,
    pub patch: Option<Operation>,
    pub trace: Option<Operation>,
}

impl PathItem {
    pub fn operations(&self) -> Vec<(&str, &Operation)> {
        let mut operations = Vec::new();

        if let Some(operation) = self.get.as_ref() {
            operations.push(("get", operation))
        }
        if let Some(operation) = self.put.as_ref() {
            operations.push(("put", operation))
        }
        if let Some(operation) = self.post.as_ref() {
            operations.push(("post", operation))
        }
        if let Some(operation) = self.delete.as_ref() {
            operations.push(("delete", operation))
        }
        if let Some(operation) = self.options.as_ref() {
            operations.push(("options", operation))
        }
        if let Some(operation) = self.head.as_ref() {
            operations.push(("head", operation))
        }
        if let Some(operation) = self.patch.as_ref() {
            operations.push(("patch", operation))
        }
        if let Some(operation) = self.trace.as_ref() {
            operations.push(("trace", operation))
        }

        operations
    }
}

#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
pub struct Operation {
    #[serde(default)]
    pub parameters: Vec<Parameter>,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct Parameter {
    pub name: String,

    #[serde(rename = "in")]
    pub location: ParameterLocation,

    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParameterLocation {
    Header,
    Query,
    Path,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Components {
    #[serde(default)]
    pub security_schemes: BTreeMap<String, SecurityScheme>,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SecurityScheme {
    ApiKey {
        name: String,

        #[serde(rename = "in")]
        location: ParameterLocation,
    },
    Http {
        scheme: String,
    },
    #[serde(rename = "oauth2")]
    OAuth2,
    OpenIdConnect {
        open_id_connect_url: String,
    },
}
pub fn extract_default_server(spec: &Spec) -> String {
    let url = spec.servers.first().map(|server| server.url.as_str()).unwrap_or("http://localhost").to_string();

    prefer_https(&url)
}

pub fn parse_spec(contents: &str) -> Result<Spec, serde_json::Error> {
    serde_json::from_str(contents)
}

pub fn extract_paths(spec: &Spec) -> &BTreeMap<String, PathItem> {
    &spec.paths
}

fn prefer_https(url: &str) -> String {
    if let Some(rest) = url.strip_prefix("http://") { format!("https://{}", rest) } else { url.to_string() }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn given_json_when_parsing_spec_then_components_are_typed() {
        let spec = parse_spec(
            r#"{
                "servers": [],
                "paths": {},
                "components": {
                    "securitySchemes": {
                        "clientIdHeader": { "type": "apiKey", "name": "X-Client-Id", "in": "header" }
                    }
                }
            }"#,
        )
        .unwrap();

        assert_eq!(
            spec.components.security_schemes.get("clientIdHeader"),
            Some(&SecurityScheme::ApiKey { name: "X-Client-Id".to_owned(), location: ParameterLocation::Header })
        );
    }

    #[test]
    fn given_json_without_components_when_parsing_then_empty_components() {
        let spec = parse_spec(r#"{ "servers": [], "paths": {} }"#).unwrap();

        assert_eq!(spec.components, Components::default());
    }

    #[test]
    fn given_spec_when_parsing_then_typed_fields_are_readable() {
        let spec = parse_spec(
            r#"{
                "servers": [{ "url": "https://api.example.com" }],
                "paths" : {
                    "/v1/users" : { "get": { "parameters": [] } }
                },
                "components": {
                    "securitySchemes": {
                        "clientIdHeader": { "type": "apiKey", "name": "X-Client-Id", "in": "header" }
                    }
                }
            }"#,
        )
        .unwrap();

        assert_eq!(spec.servers[0].url, "https://api.example.com");
    }

    #[test]
    fn given_json_when_creating_pathitem_then_pathitem() {
        let path_item: PathItem = serde_json::from_value(serde_json::json!({
            "get": {
                "parameters" : []
            },
            "put": {
                "parameters" : []
            },
            "post": {
                "parameters" : []
            },
            "delete": {
                "parameters" : []
            },
            "options": {
                "parameters" : []
            },
            "head": {
                "parameters" : []
            },
            "patch": {
                "parameters" : []
            },
            "trace": {
                "parameters" : []
            }
        }))
        .expect("valid path item");
        let operations = path_item.operations();

        assert_eq!(operations.len(), 8);
        assert_eq!(operations[0].0, "get");
        assert_eq!(operations[1].0, "put");
        assert_eq!(operations[2].0, "post");
        assert_eq!(operations[3].0, "delete");
        assert_eq!(operations[4].0, "options");
        assert_eq!(operations[5].0, "head");
        assert_eq!(operations[6].0, "patch");
        assert_eq!(operations[7].0, "trace");
    }

    #[test]
    fn given_json_when_creating_parameter_then_parameter() {
        let operation: Operation = serde_json::from_value(serde_json::json!({
            "parameters": [
                {
                    "name": "limit",
                    "in": "query",
                    "required" : true
                }
            ]
        }))
        .expect("valid parameter item");

        assert_eq!(
            operation,
            Operation {
                parameters: vec![Parameter {
                    name: "limit".to_owned(),
                    location: ParameterLocation::Query,
                    required: true,
                }],
            }
        )
    }

    #[test]
    fn given_spec_with_no_server_when_extracting_default_server_then_localhost() {
        let spec = Spec { servers: vec![], paths: BTreeMap::new(), components: Components::default() };
        assert_eq!(extract_default_server(&spec), "https://localhost")
    }

    #[test]
    fn given_spec_with_http_server_when_extracting_default_server_then_server_with_https() {
        let url = "http://api.example.com";

        let spec = Spec {
            servers: vec![Server { url: url.to_string() }],
            paths: BTreeMap::new(),
            components: Components::default(),
        };

        let expected = url.replace("http", "https");
        assert_eq!(extract_default_server(&spec), expected);
    }

    #[test]
    fn given_spec_when_extracting_default_server_then_server() {
        let url = "https://api.example.com";

        let spec = Spec {
            servers: vec![Server { url: url.to_string() }],
            paths: BTreeMap::new(),
            components: Components::default(),
        };

        assert_eq!(extract_default_server(&spec), url);
    }
}
