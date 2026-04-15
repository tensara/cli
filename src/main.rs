use dotenv::dotenv;
use serde_json::Value;
use std::path::Path;
use std::{fs, process::exit};
use tensara::{
    api::api_url,
    auth::AuthInfo,
    client::{self, ClientError},
    init::init,
    pretty::{self, pretty_print_problems},
    trpc::get_problem_by_slug,
    Parameters,
};

fn sample_endpoint_from_submit(submit_endpoint: &str) -> String {
    submit_endpoint
        .replace("/api/submissions/direct-submit", "/api/submissions/sample")
        .replace("/direct-submit", "/sample")
}

fn main() {
    #[cfg(debug_assertions)]
    dotenv().ok();

    let auth_info = AuthInfo::load();
    let parameters: Parameters = Parameters::new();

    match parameters.get_command_name().as_str() {
        "checker" | "benchmark" | "submit" | "sample" => {
            execute_problem_command(&parameters, &auth_info);
        }
        "problems" => {
            pretty_print_problems(&parameters);
        }
        "problem" => {
            execute_problem_details_command(&parameters);
        }
        "auth" => {
            execute_auth_command(&parameters);
        }
        "init" => {
            execute_init_command(&parameters);
        }
        _ => unreachable!("Invalid command type"),
    }

    // Keep this code for debugging purposes, helps to see the raw response
    // let mut response_string = String::new();
    // response.read_to_string(&mut response_string).unwrap();
    // println!("{}", response_string);
}

fn execute_problem_command(parameters: &Parameters, auth_info: &AuthInfo) {
    if !parameters.is_problem_command() {
        unreachable!("Invalid command type for problem execution");
    }

    let checker_endpoint =
        std::env::var("CHECKER_ENDPOINT").unwrap_or_else(|_| api_url("/api/submissions/checker"));
    let benchmark_endpoint = std::env::var("BENCHMARK_ENDPOINT")
        .unwrap_or_else(|_| api_url("/api/submissions/benchmark"));
    let submit_endpoint = std::env::var("SUBMIT_ENDPOINT")
        .unwrap_or_else(|_| api_url("/api/submissions/direct-submit"));
    let sample_endpoint = std::env::var("SAMPLE_ENDPOINT")
        .unwrap_or_else(|_| sample_endpoint_from_submit(&submit_endpoint));

    let command_type = parameters.get_command_name();
    let gpu_type = parameters.get_gpu_type();
    let problem_slug = parameters.get_problem_slug();
    let language = parameters.get_language();
    let dtype = parameters.get_dtype();
    let code = parameters.get_solution_code();

    if !auth_info.is_valid() {
        pretty::print_auth_error();
        exit(1);
    }

    let response = match match command_type.as_str() {
        "benchmark" => client::send_post_request_to_endpoint(
            &benchmark_endpoint,
            problem_slug,
            code,
            dtype,
            language,
            gpu_type,
            auth_info,
        ),
        "checker" => client::send_post_request_to_endpoint(
            &checker_endpoint,
            problem_slug,
            code,
            dtype,
            language,
            gpu_type,
            auth_info,
        ),
        "submit" => client::send_post_request_to_endpoint(
            &submit_endpoint,
            problem_slug,
            code,
            dtype,
            language,
            gpu_type,
            auth_info,
        ),
        "sample" => client::send_post_request_to_endpoint(
            &sample_endpoint,
            problem_slug,
            code,
            dtype,
            language,
            gpu_type,
            auth_info,
        ),
        _ => unreachable!("Invalid command type for problem execution"),
    } {
        Ok(response) => response,
        Err(ClientError::Http(error)) => {
            if error.status_code == 401 {
                pretty::print_auth_error();
            } else {
                pretty::print_http_error(&error);
            }
            exit(1);
        }
        Err(ClientError::RequestFailed(error)) => {
            pretty::print_request_error(&error);
            exit(1);
        }
    };

    match command_type.as_str() {
        "benchmark" => pretty::pretty_print_benchmark_response_v2(response, parameters),
        "checker" => pretty::pretty_print_checker_response(response, parameters),
        "submit" => pretty::pretty_print_submit_response(response),
        "sample" => pretty::pretty_print_sample_response(response),
        _ => unreachable!("Invalid command type for problem execution"),
    }
}

fn execute_auth_command(parameters: &Parameters) {
    let token = parameters.get_token();
    let auth_info = AuthInfo::new(token.unwrap().to_string(), "Tensara".to_string());
    auth_info.save();
}

fn execute_problem_details_command(parameters: &Parameters) {
    let slug = parameters.get_problem_slug();
    let problem = get_problem_by_slug(slug).unwrap_or_else(|error| {
        eprintln!("Failed to fetch problem '{}': {}", slug, error);
        exit(1);
    });

    pretty::pretty_print_problem(&problem, parameters);
}

fn execute_init_command(parameters: &Parameters) {
    let base_dir = Path::new(parameters.get_directory());
    let language = parameters.get_language();

    if parameters.get_all_flag() {
        let problems_path = dirs::home_dir()
            .expect("Could not find home directory")
            .join(".tensara")
            .join("problems.json");

        let contents = fs::read_to_string(&problems_path).expect("Could not read problems.json");

        let problems: Vec<Value> =
            serde_json::from_str(&contents).expect("Invalid problems.json format");

        for problem in problems {
            if let Some(slug) = problem.get("slug").and_then(|s| s.as_str()) {
                let subdir = base_dir.join(slug);
                fs::create_dir_all(&subdir)
                    .unwrap_or_else(|_| panic!("Failed to create directory for {}", slug));
                println!("📁 Initializing {}", slug);
                if let Err(e) = init(&subdir, language, slug) {
                    eprintln!("❌ Failed to init {}: {}", slug, e);
                }
            }
        }

        return;
    }

    let dir = parameters.get_directory();
    let slug = parameters.get_problem_slug();
    let path = Path::new(dir);
    if let Err(error) = init(path, language, slug) {
        eprintln!("Failed to initialize problem '{}': {}", slug, error);
        exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::sample_endpoint_from_submit;

    #[test]
    fn derives_sample_endpoint_from_full_submit_endpoint() {
        assert_eq!(
            sample_endpoint_from_submit("http://localhost:3000/api/submissions/direct-submit"),
            "http://localhost:3000/api/submissions/sample"
        );
    }

    #[test]
    fn derives_sample_endpoint_from_short_submit_endpoint() {
        assert_eq!(
            sample_endpoint_from_submit("https://tensara.org/direct-submit"),
            "https://tensara.org/sample"
        );
    }
}
