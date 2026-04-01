use dotenv::dotenv;
use serde_json::Value;
use std::path::Path;
use std::{fs, process::exit};
use tensara::{
    auth::AuthInfo,
    client::{self, ClientError},
    init::init,
    pretty::{self, pretty_print_problems},
    Parameters,
};

const COMPILED_CHECKER_ENDPOINT: &str = env!("COMPILED_CHECKER_ENDPOINT");
const COMPILED_BENCHMARK_ENDPOINT: &str = env!("COMPILED_BENCHMARK_ENDPOINT");
const COMPILED_SUBMIT_ENDPOINT: &str = env!("COMPILED_SUBMIT_ENDPOINT");

fn main() {
    #[cfg(debug_assertions)]
    dotenv().ok();

    let auth_info = AuthInfo::load();
    let parameters: Parameters = Parameters::new();

    match parameters.get_command_name().as_str() {
        "checker" | "benchmark" | "submit" => {
            execute_problem_command(&parameters, &auth_info);
        }
        "problems" => {
            pretty_print_problems(&parameters);
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
        std::env::var("CHECKER_ENDPOINT").unwrap_or_else(|_| COMPILED_CHECKER_ENDPOINT.to_string());
    let benchmark_endpoint = std::env::var("BENCHMARK_ENDPOINT")
        .unwrap_or_else(|_| COMPILED_BENCHMARK_ENDPOINT.to_string());
    let submit_endpoint =
        std::env::var("SUBMIT_ENDPOINT").unwrap_or_else(|_| COMPILED_SUBMIT_ENDPOINT.to_string());

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
        "benchmark" => pretty::pretty_print_benchmark_response(response),
        "checker" => pretty::pretty_print_checker_streaming_response(response),
        "submit" => pretty::pretty_print_submit_response(response),
        _ => unreachable!("Invalid command type for problem execution"),
    }
}

fn execute_auth_command(parameters: &Parameters) {
    let token = parameters.get_token();
    let auth_info = AuthInfo::new(token.unwrap().to_string(), "Tensara".to_string());
    auth_info.save();
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
    init(path, language, slug).unwrap();
}

#[cfg(test)]
mod tests {
    use tensara::auth::AuthInfo;
    use tensara::problems::is_valid_problem_slug;
    use tensara::trpc::get_problem_by_slug;

    #[test]
    fn test_is_valid_problem_slug() {
        let slug = "vector-addition";
        assert!(is_valid_problem_slug(slug));
    }

    #[test]
    fn test_is_not_valid_problem_slug() {
        let slug = "vector-adition";
        assert!(!is_valid_problem_slug(slug));
    }

    #[test]
    fn test_auth_info() {
        let auth_info = AuthInfo::new("test_token".to_string(), "Tensara".to_string());
        auth_info.save();
        let loaded_auth_info = AuthInfo::load();
        assert_eq!(auth_info.access_token, loaded_auth_info.access_token);
    }

    #[test]
    #[ignore]
    fn test_get_problem_by_slug_live() {
        let result = get_problem_by_slug("vector-addition");
        assert!(result.is_ok());
        let details = result.unwrap();

        assert_eq!(details.slug, "vector-addition");
        assert!(details.parameters.is_some());
    }
}
