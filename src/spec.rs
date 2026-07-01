use serde_json::Value;

pub fn extract_default_server(spec: &Value) -> String {
    let url = spec["servers"]
        .as_array()
        .and_then(|servers| servers.first())
        .and_then(|server| server["url"].as_str())
        .unwrap_or("http://localhost")
        .to_string();

    prefer_https(&url)
}

pub fn parse_spec(contents: &str) -> Value {
    serde_json::from_str(contents).expect("Invalid JSON")
}

pub fn extract_paths(spec: &Value) -> &serde_json::Map<String, Value> {
    spec["paths"].as_object().expect("No paths found in spec")
}

fn prefer_https(url: &str) -> String {
    if let Some(rest) = url.strip_prefix("http://") {
        format!("https://{}", rest)
    } else {
        url.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::distr::{Alphanumeric, SampleString};
    use serde_json::json;

    fn random_base_url(with_https: &bool) -> String {
        let random_string = Alphanumeric.sample_string(&mut rand::rng(), 10);
        let protocol = if *with_https { "https" } else { "http" };
        format!("{}://api-{}.com", protocol, random_string)
    }
    #[test]
    fn given_spec_with_no_server_when_extracting_default_server_then_localhost() {
        let spec = json!({});
        assert_eq!(extract_default_server(&spec), "https://localhost")
    }

    #[test]
    fn given_spec_with_http_server_when_extracting_default_server_then_server_with_https() {
        let url = random_base_url(&false);
        let spec = json!({
            "servers": [{ "url":  url }]
        });
        assert_eq!(extract_default_server(&spec), url.replace("http", "https"));
    }

    #[test]
    fn given_spec_when_extracting_default_server_then_server() {
        let url = random_base_url(&true);
        let spec = json!({
            "servers": [{ "url": url }]
        });
        assert_eq!(extract_default_server(&spec), url)
    }
}
