/*
* Use these functions to call the tRPC endpoints from the Tensara API.
*/
use crate::auth::AuthInfo;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Problem {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub difficulty: Option<String>,
    pub author: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
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

/*
* Use this function to get all problems
*/
pub fn get_all_problems() -> Result<Vec<Problem>, Box<dyn std::error::Error>> {
    let client = Client::new();
    let url = "https://tensara.org/api/trpc/problems.getAll";

    let response = client.get(url).header("User-Agent", "tensara-cli").send()?;

    let parsed: TrpcResult<Vec<Problem>> = response.json()?;
    Ok(parsed.result.data.json)
}

/*
* Function to demonstrate how to call the tRPC endpoint for user stats
*/
pub fn call_trpc_user_stats(auth: &AuthInfo) {
    let session_cookie = format!("__Secure-next-auth.session-token={}", auth.access_token);

    let client = Client::new();
    let url = "https://tensara.org/api/trpc/problems.getUserStats";

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
        "https://tensara.org/api/trpc/problems.getById?input={}",
        encoded_input
    );

    let response = client
        .get(&url)
        .header("User-Agent", "tensara-cli")
        .send()?;

    let parsed: TrpcResult<ProblemDetails> = response.json()?;
    Ok(parsed.result.data.json)
}
