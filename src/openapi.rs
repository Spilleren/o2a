use crate::spec::{Operation, Spec};

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct Parameter {
    pub name: String,

    #[serde(rename = "in")]
    pub location: ParameterLocation,

    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParameterLocation {
    Header,
    Query,
    Path,
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

pub fn spec_parameters(spec: &Spec, location: ParameterLocation) -> Vec<Parameter> {
    let paths = spec.paths.values();

    paths
        .flat_map(|path_item| path_item.operations())
        .flat_map(|(_, operation)| operation_parameters(operation, location.clone()))
        .collect()
}
pub fn operation_parameters(operation: &Operation, location: ParameterLocation) -> Vec<Parameter> {
    operation
        .parameters
        .iter()
        .filter(|parameter| parameter.location == location)
        .cloned()
        .collect()
}

pub fn header_security_schemes(spec: &Spec) -> Vec<Parameter> {
    spec.components
        .security_schemes
        .values()
        .filter_map(|scheme| match scheme {
            SecurityScheme::ApiKey { name, location } if *location == ParameterLocation::Header => {
                Some(Parameter {
                    name: name.clone(),
                    location: location.clone(),
                    required: true,
                })
            }
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::openapi::ParameterLocation::Header;
    use crate::spec::{Components, PathItem, parse_spec};

    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn given_mixed_security_schemes_when_extracting_headers_then_only_api_key_headers() {
        let spec = parse_spec(
            r#"{
                "servers": [],
                "paths": {},
                "components": {
                    "securitySchemes": {
                        "petstore_auth": { "type": "oauth2", "flows": {} },
                        "api_key": { "type": "apiKey", "name": "api_key", "in": "header" }
                    }
                }
            }"#,
        );

        let headers = header_security_schemes(&spec);

        assert_eq!(
            headers,
            vec![Parameter {
                name: "api_key".to_owned(),
                location: ParameterLocation::Header,
                required: true,
            }]
        );
    }

    #[test]
    fn given_spec_with_multiple_operations_when_extracting_header_parameters_then_returns_all_headers()
     {
        let accept_language_parameter = Parameter {
            name: "Accept-Language".to_owned(),
            location: Header,
            required: false,
        };
        let correlation_parameter = Parameter {
            name: "X-DB-Correlation-Id".to_owned(),
            location: Header,
            required: false,
        };
        let spec = Spec {
            servers: Vec::new(),

            paths: BTreeMap::from([
                (
                    "/v1/users".to_owned(),
                    PathItem {
                        get: Some(Operation {
                            parameters: vec![accept_language_parameter.clone()],
                        }),
                        ..Default::default()
                    },
                ),
                (
                    "/v1/accounts".to_owned(),
                    PathItem {
                        get: Some(Operation {
                            parameters: vec![correlation_parameter.clone()],
                        }),
                        ..Default::default()
                    },
                ),
            ]),
            components: Components::default(),
        };

        let parameters = spec_parameters(&spec, ParameterLocation::Header);

        assert_eq!(
            parameters.clone(),
            vec![correlation_parameter, accept_language_parameter]
        );
    }
}
