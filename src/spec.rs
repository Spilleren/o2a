use std::collections::BTreeMap;

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
pub enum ParameterOrRef {
    Ref {
        #[serde(rename = "$ref")]
        ref_path: String,
    },
    Inline(Parameter),
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
pub enum SecuritySchemeOrRef {
    Ref {
        #[serde(rename = "$ref")]
        ref_path: String,
    },
    Inline(SecurityScheme),
}

// ── Internal deserialization types (use OrRef) ──

#[derive(Debug, Clone, serde::Deserialize)]
pub(crate) struct RawSpec {
    #[serde(default)]
    pub servers: Vec<Server>,
    #[serde(default)]
    pub paths: BTreeMap<String, RawPathItem>,
    #[serde(default)]
    pub components: RawComponents,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub(crate) struct RawPathItem {
    #[serde(default)]
    pub parameters: Vec<ParameterOrRef>,
    pub get: Option<RawOperation>,
    pub put: Option<RawOperation>,
    pub post: Option<RawOperation>,
    pub delete: Option<RawOperation>,
    pub options: Option<RawOperation>,
    pub head: Option<RawOperation>,
    pub patch: Option<RawOperation>,
    pub trace: Option<RawOperation>,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub(crate) struct RawOperation {
    #[serde(default)]
    pub parameters: Vec<ParameterOrRef>,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawComponents {
    #[serde(default)]
    pub security_schemes: BTreeMap<String, SecuritySchemeOrRef>,
    #[serde(default)]
    pub parameters: BTreeMap<String, Parameter>,
}

// ── Public types (resolved, plain) ──

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Spec {
    #[serde(default)]
    pub servers: Vec<Server>,

    #[serde(default)]
    pub paths: BTreeMap<String, PathItem>,

    #[serde(default)]
    pub components: Components,
}

impl Spec {
    pub fn header_security_schemes(&self) -> Vec<Parameter> {
        self.components
            .security_schemes
            .values()
            .filter_map(|scheme| match scheme {
                SecurityScheme::ApiKey { name, location } if *location == ParameterLocation::Header => {
                    Some(Parameter { name: name.clone(), location: *location, required: true })
                }
                _ => None,
            })
            .collect()
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Server {
    pub url: String,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct PathItem {
    #[serde(default)]
    #[allow(dead_code)]
    pub parameters: Vec<Parameter>,
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

impl Operation {
    pub fn parameters_by(&self, location: ParameterLocation) -> Vec<Parameter> {
        self.parameters.iter().filter(|p| p.location == location).cloned().collect()
    }
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

    #[serde(default)]
    pub parameters: BTreeMap<String, Parameter>,
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

pub fn parse_spec(contents: &str) -> Result<Spec, Box<dyn std::error::Error>> {
    let raw: RawSpec = serde_json::from_str(contents)?;
    raw.resolve()
}

/// Maximum depth for resolving `$ref` chains (prevents cycles).
const MAX_REF_DEPTH: u32 = 10;

impl RawSpec {
    pub(crate) fn resolve(self) -> Result<Spec, Box<dyn std::error::Error>> {
        let resolved_schemes = resolve_security_schemes(&self.components.security_schemes)?;
        let resolved_params = self.components.parameters;

        let mut paths: BTreeMap<String, PathItem> = BTreeMap::new();
        for (path, raw_item) in self.paths {
            paths.insert(
                path,
                PathItem {
                    parameters: resolve_parameters(&raw_item.parameters, &resolved_params)?,
                    get: raw_item.get.as_ref().map(|op| resolve_operation(op, &resolved_params)).transpose()?,
                    put: raw_item.put.as_ref().map(|op| resolve_operation(op, &resolved_params)).transpose()?,
                    post: raw_item.post.as_ref().map(|op| resolve_operation(op, &resolved_params)).transpose()?,
                    delete: raw_item.delete.as_ref().map(|op| resolve_operation(op, &resolved_params)).transpose()?,
                    options: raw_item.options.as_ref().map(|op| resolve_operation(op, &resolved_params)).transpose()?,
                    head: raw_item.head.as_ref().map(|op| resolve_operation(op, &resolved_params)).transpose()?,
                    patch: raw_item.patch.as_ref().map(|op| resolve_operation(op, &resolved_params)).transpose()?,
                    trace: raw_item.trace.as_ref().map(|op| resolve_operation(op, &resolved_params)).transpose()?,
                },
            );
        }

        Ok(Spec {
            servers: self.servers,
            paths,
            components: Components { security_schemes: resolved_schemes, parameters: resolved_params },
        })
    }
}

fn resolve_operation(
    raw: &RawOperation,
    param_map: &BTreeMap<String, Parameter>,
) -> Result<Operation, Box<dyn std::error::Error>> {
    Ok(Operation { parameters: resolve_parameters(&raw.parameters, param_map)? })
}

fn resolve_parameters(
    raw_params: &[ParameterOrRef],
    param_map: &BTreeMap<String, Parameter>,
) -> Result<Vec<Parameter>, Box<dyn std::error::Error>> {
    let mut resolved = Vec::with_capacity(raw_params.len());
    for item in raw_params {
        match item {
            ParameterOrRef::Inline(p) => resolved.push(p.clone()),
            ParameterOrRef::Ref { ref_path } => match resolve_param_ref(ref_path, param_map) {
                Ok(p) => resolved.push(p.clone()),
                Err(msg) => eprintln!("warning: {msg}"),
            },
        }
    }
    Ok(resolved)
}

fn resolve_param_ref(ref_path: &str, param_map: &BTreeMap<String, Parameter>) -> Result<Parameter, String> {
    let segments = parse_json_pointer(ref_path)?;
    if segments.len() != 3 || segments[0] != "components" || segments[1] != "parameters" {
        return Err(format!("unsupported $ref path '{ref_path}': only #/components/parameters/... is supported"));
    }
    let name = &segments[2];
    param_map.get(name).cloned().ok_or_else(|| format!("unresolvable $ref '{ref_path}': parameter '{name}' not found"))
}

fn resolve_security_schemes(
    raw: &BTreeMap<String, SecuritySchemeOrRef>,
) -> Result<BTreeMap<String, SecurityScheme>, Box<dyn std::error::Error>> {
    let mut resolved: BTreeMap<String, SecurityScheme> = BTreeMap::new();
    for (name, item) in raw {
        match item {
            SecuritySchemeOrRef::Inline(s) => {
                resolved.insert(name.clone(), s.clone());
            }
            SecuritySchemeOrRef::Ref { ref_path } => {
                if let Some(s) = resolve_security_scheme_ref(ref_path, raw, 0)? {
                    resolved.insert(name.clone(), s);
                }
            }
        }
    }
    Ok(resolved)
}

fn resolve_security_scheme_ref(
    ref_path: &str,
    all_schemes: &BTreeMap<String, SecuritySchemeOrRef>,
    depth: u32,
) -> Result<Option<SecurityScheme>, Box<dyn std::error::Error>> {
    if depth > MAX_REF_DEPTH {
        return Err(format!("$ref depth limit ({MAX_REF_DEPTH}) exceeded resolving '{ref_path}'").into());
    }

    let segments = parse_json_pointer(ref_path).map_err(|e| format!("invalid $ref '{ref_path}': {e}"))?;
    if segments.len() != 3 || segments[0] != "components" || segments[1] != "securitySchemes" {
        return Err(
            format!("unsupported $ref path '{ref_path}': only #/components/securitySchemes/... is supported").into()
        );
    }

    let name = &segments[2];
    let item = match all_schemes.get(name) {
        Some(item) => item,
        None => {
            eprintln!("warning: unresolvable $ref '{ref_path}': scheme '{name}' not found");
            return Ok(None);
        }
    };

    match item {
        SecuritySchemeOrRef::Inline(s) => Ok(Some(s.clone())),
        SecuritySchemeOrRef::Ref { ref_path: next_ref } => {
            resolve_security_scheme_ref(next_ref, all_schemes, depth + 1)
        }
    }
}

/// Parse a JSON Pointer (RFC 6901) into path segments.
/// Only handles internal `#/...` refs.
fn parse_json_pointer(ref_path: &str) -> Result<Vec<String>, String> {
    let ptr = ref_path.strip_prefix('#').ok_or_else(|| format!("$ref '{ref_path}' must start with '#'"))?;
    if ptr.is_empty() {
        return Ok(vec!["".to_owned()]);
    }
    if !ptr.starts_with('/') {
        return Err(format!("$ref '{ref_path}' has invalid pointer syntax after '#'"));
    }

    let segments: Vec<String> = ptr[1..].split('/').map(|seg| seg.replace("~1", "/").replace("~0", "~")).collect();

    Ok(segments)
}

pub(crate) fn prefer_https(url: &str) -> String {
    if let Some(rest) = url.strip_prefix("http://") { format!("https://{}", rest) } else { url.to_string() }
}

pub fn bash_variable_name(name: &str) -> String {
    name.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_uppercase() } else { '_' }).collect()
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    mod parameter_or_ref_tests {
        use super::*;

        #[test]
        fn given_ref_json_when_deserializing_then_ref_variant() {
            let json = r##"{"$ref": "#/components/parameters/UserFilter"}"##;
            let parsed: ParameterOrRef = serde_json::from_str(json).unwrap();

            match parsed {
                ParameterOrRef::Ref { ref_path } => {
                    assert_eq!(ref_path, "#/components/parameters/UserFilter");
                }
                ParameterOrRef::Inline(_) => panic!("expected Ref variant, got Inline"),
            }
        }

        #[test]
        fn given_inline_json_when_deserializing_then_inline_variant() {
            let json = r##"{"name": "limit", "in": "query", "required": true}"##;
            let parsed: ParameterOrRef = serde_json::from_str(json).unwrap();

            match parsed {
                ParameterOrRef::Inline(param) => {
                    assert_eq!(param.name, "limit");
                    assert_eq!(param.location, ParameterLocation::Query);
                    assert!(param.required);
                }
                ParameterOrRef::Ref { .. } => panic!("expected Inline variant, got Ref"),
            }
        }

        #[test]
        fn given_ref_with_slash_escapes_when_deserializing_then_preserves_raw_path() {
            let json = r##"{"$ref": "#/components/parameters/foo~1bar~0baz"}"##;
            let parsed: ParameterOrRef = serde_json::from_str(json).unwrap();

            match parsed {
                ParameterOrRef::Ref { ref_path } => {
                    assert_eq!(ref_path, "#/components/parameters/foo~1bar~0baz");
                }
                ParameterOrRef::Inline(_) => panic!("expected Ref variant, got Inline"),
            }
        }
    }

    mod security_scheme_or_ref_tests {
        use super::*;

        #[test]
        fn given_ref_json_when_deserializing_then_ref_variant() {
            let json = r##"{"$ref": "#/components/securitySchemes/BasicAuth"}"##;
            let parsed: SecuritySchemeOrRef = serde_json::from_str(json).unwrap();

            match parsed {
                SecuritySchemeOrRef::Ref { ref_path } => {
                    assert_eq!(ref_path, "#/components/securitySchemes/BasicAuth");
                }
                SecuritySchemeOrRef::Inline(_) => panic!("expected Ref variant, got Inline"),
            }
        }

        #[test]
        fn given_inline_json_when_deserializing_then_inline_variant() {
            let json = r#"{"type": "apiKey", "name": "X-API-Key", "in": "header"}"#;
            let parsed: SecuritySchemeOrRef = serde_json::from_str(json).unwrap();

            match parsed {
                SecuritySchemeOrRef::Inline(scheme) => {
                    assert_eq!(
                        scheme,
                        SecurityScheme::ApiKey { name: "X-API-Key".to_owned(), location: ParameterLocation::Header }
                    );
                }
                SecuritySchemeOrRef::Ref { .. } => panic!("expected Inline variant, got Ref"),
            }
        }
    }

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
    fn given_name_with_hyphens_when_normalizing_then_uses_uppercase_underscores() {
        assert_eq!(bash_variable_name("X-DB-Correlation-Id"), "X_DB_CORRELATION_ID");
    }

    #[test]
    fn given_camel_case_name_when_normalizing_then_uppercases_without_extra_underscores() {
        assert_eq!(bash_variable_name("accountFilterId"), "ACCOUNTFILTERID");
    }

    mod resolve_tests {
        use super::*;
        use std::collections::BTreeMap;

        fn make_raw_spec_with_param_ref(ref_path: &str) -> RawSpec {
            RawSpec {
                servers: vec![],
                paths: BTreeMap::from([(
                    "/users".to_owned(),
                    RawPathItem {
                        get: Some(RawOperation {
                            parameters: vec![ParameterOrRef::Ref { ref_path: ref_path.to_owned() }],
                        }),
                        ..Default::default()
                    },
                )]),
                components: RawComponents {
                    security_schemes: BTreeMap::new(),
                    parameters: BTreeMap::from([(
                        "UserFilter".to_owned(),
                        Parameter { name: "filter".to_owned(), location: ParameterLocation::Query, required: true },
                    )]),
                },
            }
        }

        #[test]
        fn given_operation_param_ref_when_resolving_then_reference_is_inlined() {
            let raw = make_raw_spec_with_param_ref("#/components/parameters/UserFilter");
            let spec = raw.resolve().unwrap();

            let op = spec.paths.get("/users").unwrap().get.as_ref().unwrap();
            assert_eq!(
                op.parameters,
                vec![Parameter { name: "filter".to_owned(), location: ParameterLocation::Query, required: true }]
            );
        }

        #[test]
        fn given_security_scheme_ref_when_resolving_then_reference_is_inlined() {
            let raw = RawSpec {
                servers: vec![],
                paths: BTreeMap::new(),
                components: RawComponents {
                    security_schemes: BTreeMap::from([
                        (
                            "main".to_owned(),
                            SecuritySchemeOrRef::Ref { ref_path: "#/components/securitySchemes/target".to_owned() },
                        ),
                        (
                            "target".to_owned(),
                            SecuritySchemeOrRef::Inline(SecurityScheme::ApiKey {
                                name: "X-Key".to_owned(),
                                location: ParameterLocation::Header,
                            }),
                        ),
                    ]),
                    parameters: BTreeMap::new(),
                },
            };
            let spec = raw.resolve().unwrap();
            let resolved = spec.components.security_schemes.get("main").unwrap();
            assert_eq!(
                resolved,
                &SecurityScheme::ApiKey { name: "X-Key".to_owned(), location: ParameterLocation::Header }
            );
        }

        #[test]
        fn given_unresolvable_param_ref_when_resolving_then_item_is_skipped() {
            let mut raw = make_raw_spec_with_param_ref("#/components/parameters/DoesNotExist");
            // Also include a valid inline parameter to ensure we don't lose it
            raw.paths.get_mut("/users").unwrap().get.as_mut().unwrap().parameters.push(ParameterOrRef::Inline(
                Parameter { name: "limit".to_owned(), location: ParameterLocation::Query, required: false },
            ));

            let spec = raw.resolve().unwrap();
            let op = spec.paths.get("/users").unwrap().get.as_ref().unwrap();
            // The unresolvable ref is skipped; only the inline parameter remains
            assert_eq!(
                op.parameters,
                vec![Parameter { name: "limit".to_owned(), location: ParameterLocation::Query, required: false }]
            );
        }

        #[test]
        fn given_path_item_parameters_when_resolving_then_are_resolved() {
            let raw = RawSpec {
                servers: vec![],
                paths: BTreeMap::from([(
                    "/users".to_owned(),
                    RawPathItem {
                        parameters: vec![ParameterOrRef::Ref {
                            ref_path: "#/components/parameters/AcceptLang".to_owned(),
                        }],
                        ..Default::default()
                    },
                )]),
                components: RawComponents {
                    security_schemes: BTreeMap::new(),
                    parameters: BTreeMap::from([(
                        "AcceptLang".to_owned(),
                        Parameter {
                            name: "Accept-Language".to_owned(),
                            location: ParameterLocation::Header,
                            required: false,
                        },
                    )]),
                },
            };
            let spec = raw.resolve().unwrap();
            let path_item = spec.paths.get("/users").unwrap();
            assert_eq!(
                path_item.parameters,
                vec![Parameter {
                    name: "Accept-Language".to_owned(),
                    location: ParameterLocation::Header,
                    required: false,
                }]
            );
        }

        #[test]
        fn given_circular_security_scheme_refs_when_resolving_then_errors_at_depth_limit() {
            // A → B → A creates a cycle detected by depth cap
            let raw = RawSpec {
                servers: vec![],
                paths: BTreeMap::new(),
                components: RawComponents {
                    security_schemes: BTreeMap::from([
                        (
                            "a".to_owned(),
                            SecuritySchemeOrRef::Ref { ref_path: "#/components/securitySchemes/b".to_owned() },
                        ),
                        (
                            "b".to_owned(),
                            SecuritySchemeOrRef::Ref { ref_path: "#/components/securitySchemes/a".to_owned() },
                        ),
                    ]),
                    parameters: BTreeMap::new(),
                },
            };
            let result = raw.resolve();
            assert!(result.is_err(), "expected depth overflow error, got Ok");
            assert!(result.unwrap_err().to_string().contains("depth"), "error should mention depth");
        }

        #[test]
        fn given_deep_but_valid_chain_when_resolving_then_succeeds() {
            // Chain of 9 (within depth limit of 10)
            let mut schemes = BTreeMap::new();
            for i in 0..9 {
                let key = format!("level{i}");
                if i == 8 {
                    schemes.insert(
                        key,
                        SecuritySchemeOrRef::Inline(SecurityScheme::ApiKey {
                            name: "final".to_owned(),
                            location: ParameterLocation::Header,
                        }),
                    );
                } else {
                    schemes.insert(
                        key,
                        SecuritySchemeOrRef::Ref { ref_path: format!("#/components/securitySchemes/level{}", i + 1) },
                    );
                }
            }
            let raw = RawSpec {
                servers: vec![],
                paths: BTreeMap::new(),
                components: RawComponents { security_schemes: schemes, parameters: BTreeMap::new() },
            };
            let spec = raw.resolve().unwrap();
            let resolved = spec.components.security_schemes.get("level0").unwrap();
            assert_eq!(
                resolved,
                &SecurityScheme::ApiKey { name: "final".to_owned(), location: ParameterLocation::Header }
            );
        }
    }

    mod parse_spec_ref_tests {
        use super::*;

        #[test]
        fn given_json_with_param_ref_when_parse_spec_then_resolves_ref() {
            let json = r##"{
                "servers": [{"url": "https://api.example.com"}],
                "paths": {
                    "/users": {
                        "get": {
                            "parameters": [
                                {"$ref": "#/components/parameters/UserFilter"}
                            ]
                        }
                    }
                },
                "components": {
                    "parameters": {
                        "UserFilter": {
                            "name": "filter",
                            "in": "query",
                            "required": true
                        }
                    }
                }
            }"##;

            let spec = parse_spec(json).unwrap();
            let op = spec.paths.get("/users").unwrap().get.as_ref().unwrap();

            assert_eq!(
                op.parameters,
                vec![Parameter { name: "filter".to_owned(), location: ParameterLocation::Query, required: true }]
            );
        }

        #[test]
        fn given_json_with_security_scheme_ref_when_parse_spec_then_resolves_ref() {
            let json = r##"{
                "servers": [],
                "paths": {},
                "components": {
                    "securitySchemes": {
                        "main": {"$ref": "#/components/securitySchemes/target"},
                        "target": {"type": "apiKey", "name": "X-Key", "in": "header"}
                    }
                }
            }"##;

            let spec = parse_spec(json).unwrap();
            let resolved = spec.components.security_schemes.get("main").unwrap();
            assert_eq!(
                resolved,
                &SecurityScheme::ApiKey { name: "X-Key".to_owned(), location: ParameterLocation::Header }
            );
        }
    }
}
