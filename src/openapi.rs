#[derive(Debug, Clone, serde::Deserialize)]
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

pub fn spec_parameters(
    spec: &serde_json::Value,
    location: ParameterLocation,
) -> Vec<Parameter> {
    let Some(paths) = spec["paths"].as_object() else {
        return Vec::new();
    };

    paths
        .values()
        .filter_map(|path_item| path_item.as_object())
        .flat_map(|methods| methods.values())
        .flat_map(|operation| operation_parameters(operation, location.clone()))
        .collect()
}
pub fn operation_parameters(
    operation: &serde_json::Value,
    location: ParameterLocation,
) -> Vec<Parameter> {
    operation["parameters"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|value| serde_json::from_value::<Parameter>(value).ok())
        .filter(|parameter| parameter.location == location)
        .collect()
}

pub fn header_security_schemes(spec: &serde_json::Value) -> Vec<Parameter> {
    let Some(security_schemes) = spec["components"]["securitySchemes"].as_object() else {
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
#[test]
fn given_spec_with_multiple_operations_when_extracting_header_parameters_then_returns_all_headers()
{
    let spec = serde_json::json!({
        "paths": {
            "/v1/users": {
                "get": {
                    "parameters": [
                        { "name": "Accept-Language", "in": "header" },
                        { "name": "limit", "in": "query" }
                    ]
                }
            },
            "/v1/accounts": {
                "get": {
                    "parameters": [
                        { "name": "X-DB-Correlation-Id", "in": "header" }
                    ]
                }
            }
        }
    });

    let parameters = spec_parameters(&spec, ParameterLocation::Header);

    let mut names = parameters
        .into_iter()
        .map(|parameter| parameter.name)
        .collect::<Vec<_>>();
    names.sort();

    assert_eq!(names, vec!["Accept-Language", "X-DB-Correlation-Id"]);
}
