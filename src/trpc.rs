/*
* Use these functions to call the tRPC endpoints from the Tensara API.
*/
use crate::{
    api::{api_base_url, api_url},
    auth::AuthInfo,
};
use reqwest::blocking::Client;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{error::Error, fmt};

#[derive(Debug)]
struct TrpcFetchError(String);

impl fmt::Display for TrpcFetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for TrpcFetchError {}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Problem {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub difficulty: Option<String>,
    pub author: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProblemDetails {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub difficulty: Option<String>,
    pub author: Option<String>,
    pub tags: Option<Vec<String>>,
    pub description: Option<String>,
    pub definition: Option<String>,
    pub parameters: Option<Vec<ProblemParameter>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProblemParameter {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
    #[serde(rename = "const", default)]
    pub const_: Option<String>,
    #[serde(default)]
    pub pointer: Option<String>,
    #[serde(default)]
    pub constant: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSubmissionInput {
    pub problem_slug: String,
    pub code: String,
    pub language: String,
    pub gpu_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Submission {
    pub id: String,
    pub status: Option<String>,
    pub language: Option<String>,
    pub gpu_type: Option<String>,
    pub problem_id: String,
    pub user_id: String,
}

#[derive(Debug, Deserialize)]
struct TrpcResult<T> {
    result: TrpcData<T>,
}

#[derive(Debug, Deserialize)]
struct TrpcData<T> {
    data: TrpcJson<T>,
}

#[derive(Debug, Deserialize)]
struct TrpcJson<T> {
    json: T,
}

fn truncate_body(body: &str) -> String {
    const MAX_CHARS: usize = 500;
    let trimmed = body.trim();

    if trimmed.chars().count() <= MAX_CHARS {
        return trimmed.to_string();
    }

    format!("{}...", trimmed.chars().take(MAX_CHARS).collect::<String>())
}

fn fetch_trpc_json<T: DeserializeOwned>(client: &Client, url: &str) -> Result<T, Box<dyn Error>> {
    let response = client
        .get(url)
        .header("User-Agent", "tensara-cli")
        .send()
        .map_err(|error| {
            TrpcFetchError(format!(
                "failed to connect to Tensara API at {}: {}",
                url, error
            ))
        })?;

    let status = response.status();
    let body = response.text().map_err(|error| {
        TrpcFetchError(format!(
            "failed to read Tensara API response from {}: {}",
            url, error
        ))
    })?;

    if !status.is_success() {
        let body = truncate_body(&body);
        let suffix = if body.is_empty() {
            String::new()
        } else {
            format!(": {}", body)
        };

        return Err(Box::new(TrpcFetchError(format!(
            "Tensara API returned HTTP {} for {}{}",
            status, url, suffix
        ))));
    }

    serde_json::from_str(&body).map_err(|error| {
        Box::new(TrpcFetchError(format!(
            "failed to parse Tensara API JSON from {}: {}. Response body: {}",
            url,
            error,
            truncate_body(&body)
        ))) as Box<dyn Error>
    })
}

/*
* Use this function to get all problems
*/
pub fn get_all_problems() -> Result<Vec<Problem>, Box<dyn std::error::Error>> {
    let client = Client::new();
    let url = api_url("/api/trpc/problems.getAll");

    let parsed: TrpcResult<Vec<Problem>> = fetch_trpc_json(&client, &url)?;
    Ok(parsed.result.data.json)
}

/*
* Function to demonstrate how to call the tRPC endpoint for user stats
*/
pub fn call_trpc_user_stats(auth: &AuthInfo) {
    let session_cookie = format!("__Secure-next-auth.session-token={}", auth.access_token);

    let client = Client::new();
    let url = api_url("/api/trpc/problems.getUserStats");

    let response = client
        .get(url)
        .header("Cookie", session_cookie)
        .header("User-Agent", "tensara-cli")
        .send()
        .expect("Failed to send request");

    let text = response.text().unwrap();
    println!("tRPC Response: {}", text);
}

/*
* Use this function to get the problem details by slug
*/
pub fn get_problem_by_slug(slug: &str) -> Result<ProblemDetails, Box<dyn std::error::Error>> {
    let client = Client::new();
    let input_json = serde_json::json!({ "json": { "slug": slug } }).to_string();
    let encoded_input = urlencoding::encode(&input_json).into_owned();
    let url = format!(
        "{}/api/trpc/problems.getById?input={}",
        api_base_url(),
        encoded_input
    );

    let parsed: TrpcResult<ProblemDetails> = fetch_trpc_json(&client, &url)?;
    Ok(parsed.result.data.json)
}
