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

#[derive(Debug, Clone, serde::Deserialize)]
pub struct SecurityScheme {
    #[serde(rename = "type")]
    pub scheme_type: SecuritySchemeType,

    pub name: String,

    #[serde(rename = "in")]
    pub location: ParameterLocation,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SecuritySchemeType {
    ApiKey,
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
    let Some(security_schemes) = spec.components["securitySchemes"].as_object() else {
        return Vec::new();
    };

    security_schemes
        .values()
        .filter_map(|value| serde_json::from_value::<SecurityScheme>(value.clone()).ok())
        .filter(|scheme| {
            scheme.scheme_type == SecuritySchemeType::ApiKey
                && scheme.location == ParameterLocation::Header
        })
        .map(|scheme| Parameter {
            name: scheme.name,
            location: scheme.location,
            required: true,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::openapi::ParameterLocation::Header;
    use crate::spec::PathItem;

    use super::*;
    use std::collections::BTreeMap;

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
            components: serde_json::json!({}),
        };

        let parameters = spec_parameters(&spec, ParameterLocation::Header);

        assert_eq!(
            parameters.clone(),
            vec![correlation_parameter, accept_language_parameter]
        );
    }
}
