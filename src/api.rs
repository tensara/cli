const DEFAULT_API_BASE_URL: &str = "https://tensara.org";

fn normalize_api_base_url(value: Option<String>) -> String {
    value
        .unwrap_or_else(|| DEFAULT_API_BASE_URL.to_string())
        .trim()
        .trim_end_matches('/')
        .to_string()
}

pub fn api_base_url() -> String {
    normalize_api_base_url(std::env::var("TENSARA_API_BASE_URL").ok())
}

pub fn api_url(path: &str) -> String {
    format!("{}/{}", api_base_url(), path.trim_start_matches('/'))
}

#[cfg(test)]
mod tests {
    use super::normalize_api_base_url;

    #[test]
    fn normalizes_configured_api_base_url() {
        assert_eq!(
            normalize_api_base_url(Some(" http://localhost:3000/ ".to_string())),
            "http://localhost:3000"
        );
    }

    #[test]
    fn defaults_to_production_api_base_url() {
        assert_eq!(normalize_api_base_url(None), "https://tensara.org");
    }
}
