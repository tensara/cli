use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

use crate::auth::AuthInfo;

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize)]
struct Request {
    problemSlug: String,
    code: String,
    dtype: String,
    language: String,
    gpuType: String,
}

#[allow(non_snake_case)]
impl Request {
    pub fn new(
        problemSlug: String,
        code: String,
        dtype: String,
        language: String,
        gpuType: String,
    ) -> Self {
        Self {
            problemSlug,
            code,
            dtype,
            language,
            gpuType,
        }
    }
}

#[derive(Debug)]
pub enum ClientError {
    RequestFailed(String),
    Http(HttpError),
}

#[derive(Debug)]
pub struct HttpError {
    pub endpoint: String,
    pub status_code: u16,
    pub status_text: String,
    pub content_type: Option<String>,
    pub error: Option<String>,
    pub message: Option<String>,
    pub details: Option<String>,
    pub raw_body: String,
}

#[derive(Debug, Deserialize)]
struct ErrorPayload {
    error: Option<String>,
    message: Option<String>,
    details: Option<String>,
    status: Option<String>,
}

pub fn send_get_request(endpoint: &str) {
    let client = Client::new();
    match client.get(endpoint).send() {
        Ok(response) => {
            if response.status().is_success() {
                match response.text() {
                    Ok(text) => println!("{}", text),
                    Err(e) => println!("Error reading response: {}", e),
                }
            } else {
                println!("Error: {}", response.status());
            }
        }
        Err(e) => println!("Request failed: {}", e),
    }
}

pub fn send_post_request_to_endpoint(
    endpoint: &str,
    problem_slug: &str,
    code: &str,
    dtype: &str,
    language: &str,
    gpu_type: &str,
    auth: &AuthInfo,
) -> Result<reqwest::blocking::Response, ClientError> {
    let request = Request::new(
        problem_slug.to_string(),
        code.to_string(),
        dtype.to_string(),
        language.to_string(),
        gpu_type.to_string(),
    );
    let request_json = serde_json::to_string(&request).unwrap();
    let client = Client::new();
    let response = client
        .post(endpoint)
        .header("Content-Type", "application/json")
        .header("User-Agent", "tensara-cli")
        .header("Authorization", format!("Bearer {}", auth.access_token))
        .body(request_json)
        .send()
        .map_err(|e| ClientError::RequestFailed(e.to_string()))?;

    if response.status().is_success() {
        return Ok(response);
    }

    let status = response.status();
    let status_code = status.as_u16();
    let status_text = status.to_string();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.to_string());
    let raw_body = response.text().unwrap_or_default();
    let parsed = serde_json::from_str::<ErrorPayload>(&raw_body).ok();

    Err(ClientError::Http(HttpError {
        endpoint: endpoint.to_string(),
        status_code,
        status_text,
        content_type,
        error: parsed.as_ref().and_then(|p| p.error.clone()),
        message: parsed
            .as_ref()
            .and_then(|p| p.message.clone().or_else(|| p.status.clone())),
        details: parsed.as_ref().and_then(|p| p.details.clone()),
        raw_body,
    }))
}
