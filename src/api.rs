const DEFAULT_API_BASE_URL: &str = "https://tensara.org";

pub fn api_base_url() -> String {
    std::env::var("TENSARA_API_BASE_URL")
        .unwrap_or_else(|_| DEFAULT_API_BASE_URL.to_string())
        .trim()
        .trim_end_matches('/')
        .to_string()
}

pub fn api_url(path: &str) -> String {
    format!("{}/{}", api_base_url(), path.trim_start_matches('/'))
}
