use crate::{
    client::HttpError,
    trpc::{get_all_problems, ProblemDetails},
    Parameters,
};
use colored::*;
use console::style;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use serde::Deserialize;
use serde_json::Value;
use std::io::Read;
use std::io::{BufRead, BufReader};
use std::thread;
use std::time::Duration;

pub fn pretty_print_problems(parameters: &Parameters) {
    let mut problems = get_all_problems().unwrap_or_else(|_| {
        eprintln!("Failed to fetch problems.");
        std::process::exit(1);
    });

    if parameters.get_json_output_flag() {
        println!(
            "{}",
            serde_json::to_string_pretty(&problems).expect("Failed to serialize problems")
        );
        return;
    }

    println!("Fetching problems...");
    let fields = parameters
        .get_fields()
        .cloned()
        .unwrap_or_else(|| vec!["slug".to_string(), "title".to_string()]);
    let sort_by = parameters.get_sort_by().cloned();

    if let Some(sort_field) = sort_by {
        match sort_field.as_str() {
            "slug" => problems.sort_by(|a, b| a.slug.cmp(&b.slug)),
            "title" => problems.sort_by(|a, b| a.title.cmp(&b.title)),
            "difficulty" => problems.sort_by(|a, b| a.difficulty.cmp(&b.difficulty)),
            "author" => problems.sort_by(|a, b| a.author.cmp(&b.author)),
            _ => {
                eprintln!("Invalid sort field: {}", sort_field);
            }
        }
    }

    let max_slug_length = problems.iter().map(|p| p.slug.len()).max().unwrap_or(0);

    for problem in problems.iter() {
        let slug = format!(
            "{:<width$}",
            problem.slug.bold(),
            width = max_slug_length + 2
        );
        let mut difficulty = String::new();
        let mut author = String::new();
        let mut tags = String::new();

        for field in &fields {
            match field.as_str() {
                "difficulty" => {
                    if let Some(diff) = &problem.difficulty {
                        let colored = match diff.as_str() {
                            "EASY" => diff.green(),
                            "MEDIUM" => diff.yellow(),
                            "HARD" => diff.red(),
                            _ => diff.normal(),
                        };
                        let pad_width = 8 - diff.len();
                        difficulty = format!("[{}]{}", colored, " ".repeat(pad_width));
                    }
                }
                "author" => {
                    if let Some(a) = &problem.author {
                        author = format!(" by {}", a.dimmed());
                    }
                }
                "tags" => {
                    if let Some(t) = &problem.tags {
                        tags = format!(" ({})", t.join(", "));
                    }
                }
                _ => {}
            }
        }

        let view_link = format!(
            "\x1b]8;;https://tensara.org/problems/{}\x1b\\{}\x1b]8;;\x1b\\",
            problem.slug,
            "(view)".blue().underline()
        );

        println!("{} {} {} {}{}", slug, difficulty, author, tags, view_link);
    }
}

fn extract_reference_solution(definition: &str) -> Option<String> {
    let lines: Vec<&str> = definition.lines().collect();

    for (start, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with("def reference_solution(") {
            continue;
        }

        let base_indent = line.len() - trimmed.len();
        let mut end = start + 1;

        while end < lines.len() {
            let next = lines[end];
            let next_trimmed = next.trim_start();
            if next_trimmed.is_empty() {
                end += 1;
                continue;
            }

            let next_indent = next.len() - next_trimmed.len();
            if next_indent <= base_indent && !next_trimmed.starts_with('@') {
                break;
            }
            end += 1;
        }

        return Some(lines[start..end].join("\n"));
    }

    None
}

fn problem_json(problem: &ProblemDetails) -> Value {
    let mut value = serde_json::to_value(problem).expect("Failed to serialize problem");

    if let Some(object) = value.as_object_mut() {
        object.insert(
            "reference_solution".to_string(),
            problem
                .definition
                .as_deref()
                .and_then(extract_reference_solution)
                .map(Value::String)
                .unwrap_or(Value::Null),
        );
    }

    value
}

#[cfg(test)]
mod tests {
    use super::extract_reference_solution;

    #[test]
    fn extracts_reference_solution_method_only() {
        let definition = r#"
class VectorAddition:
    def reference_solution(self, a, b):
        c = a + b
        return c

    def verify_result(self, expected, actual):
        return True, {}
"#;

        let reference = extract_reference_solution(definition).unwrap();

        assert!(reference.contains("def reference_solution"));
        assert!(reference.contains("return c"));
        assert!(!reference.contains("def verify_result"));
    }

    #[test]
    fn returns_none_when_reference_solution_is_missing() {
        assert!(extract_reference_solution("class Problem:\n    pass").is_none());
    }
}

pub fn pretty_print_problem(problem: &ProblemDetails, parameters: &Parameters) {
    if parameters.get_json_output_flag() {
        println!(
            "{}",
            serde_json::to_string_pretty(&problem_json(problem))
                .expect("Failed to serialize problem")
        );
        return;
    }

    let show_description = !parameters.get_reference_only_flag();
    let show_reference = !parameters.get_description_only_flag();

    println!("{}", style(&problem.title).green().bold());
    println!("{}", style(&problem.slug).dim());

    if let Some(difficulty) = &problem.difficulty {
        println!("Difficulty: {}", difficulty);
    }
    if let Some(author) = &problem.author {
        println!("Author: {}", author);
    }
    if let Some(tags) = &problem.tags {
        if !tags.is_empty() {
            println!("Tags: {}", tags.join(", "));
        }
    }

    if show_description {
        println!("\n{}", style("Description").bold().underlined());
        match problem.description.as_deref() {
            Some(description) if !description.trim().is_empty() => {
                println!("{}", description.trim())
            }
            _ => println!("{}", style("No description available.").yellow()),
        }
    }

    if !parameters.get_reference_only_flag() {
        if let Some(problem_parameters) = &problem.parameters {
            if !problem_parameters.is_empty() {
                println!("\n{}", style("Parameters").bold().underlined());
                for parameter in problem_parameters {
                    let mut attrs = vec![parameter.ty.clone()];
                    if parameter.pointer.as_deref() == Some("true") {
                        attrs.push("pointer".to_string());
                    }
                    if parameter.constant.as_deref() == Some("true") {
                        attrs.push("const".to_string());
                    }
                    println!(
                        "{}: {}",
                        style(&parameter.name).cyan().bold(),
                        attrs.join(", ")
                    );
                }
            }
        }
    }

    if show_reference {
        println!("\n{}", style("PyTorch Reference").bold().underlined());
        match problem
            .definition
            .as_deref()
            .and_then(extract_reference_solution)
        {
            Some(reference) => {
                println!("{}", reference.trim());
            }
            None => println!(
                "{}",
                style("No reference_solution function found in the problem definition.").yellow()
            ),
        }
    }
}

fn print_json_section(label: &str, value: Option<&Value>) {
    if let Some(value) = value {
        println!("\n{}", style(label).bold().underlined());
        println!(
            "{}",
            serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
        );
    }
}

pub fn pretty_print_sample_response(response: impl Read) {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(default_spinner_style());
    spinner.set_message("Running sample...");
    spinner.enable_steady_tick(Duration::from_millis(80));

    let reader = BufReader::new(response);

    for line in reader.lines().flatten() {
        spinner.tick();

        if !line.starts_with("data: ") {
            continue;
        }

        let json_data = &line[6..];
        let Ok(json) = serde_json::from_str::<Value>(json_data) else {
            continue;
        };

        match json.get("status").and_then(|s| s.as_str()) {
            Some("PASSED") => {
                spinner.finish_and_clear();
                println!("{}", style("✅ Sample Passed").green().bold());
                print_json_section("Input", json.get("input"));
                print_json_section("Expected Output", json.get("expected_output"));
                print_json_section("Actual Output", json.get("output"));
                if let Some(stdout) = json.get("stdout").and_then(|v| v.as_str()) {
                    if !stdout.trim().is_empty() {
                        println!("\n{}", style("Stdout").bold().underlined());
                        println!("{}", stdout.trim());
                    }
                }
                if let Some(stderr) = json.get("stderr").and_then(|v| v.as_str()) {
                    if !stderr.trim().is_empty() {
                        println!("\n{}", style("Stderr").bold().underlined());
                        println!("{}", style(stderr.trim()).yellow());
                    }
                }
                return;
            }
            Some("FAILED") => {
                spinner.finish_and_clear();
                println!("{}", style("❌ Sample Failed").red().bold());
                print_json_section("Input", json.get("input"));
                print_json_section("Expected Output", json.get("expected_output"));
                print_json_section("Actual Output", json.get("output"));
                print_json_section("Debug Info", json.get("debug_info"));
                return;
            }
            Some("COMPILE_ERROR")
            | Some("RUNTIME_ERROR")
            | Some("TIME_LIMIT_EXCEEDED")
            | Some("TOO_MANY_REQUESTS")
            | Some("RATE_LIMIT_EXCEEDED")
            | Some("SANDBOX_TIMEOUT")
            | Some("SANDBOX_OUTPUT_LIMIT")
            | Some("OUTPUT_LIMIT_EXCEEDED")
            | Some("ERROR") => {
                spinner.finish_and_clear();
                let status = json
                    .get("status")
                    .and_then(|s| s.as_str())
                    .unwrap_or("ERROR");
                let message = json
                    .get("message")
                    .and_then(|m| m.as_str())
                    .or_else(|| json.get("error").and_then(|m| m.as_str()))
                    .unwrap_or("Sample run failed");
                println!("{}: {}", style(status).red().bold(), message);
                if let Some(details) = json.get("details").and_then(|d| d.as_str()) {
                    println!("\n{}", style("Details").bold().underlined());
                    println!("{}", details);
                }
                return;
            }
            Some(status) => {
                spinner.set_message(status.to_string());
            }
            None => {}
        }
    }

    spinner.finish_and_clear();
    println!(
        "{}",
        style("Sample stream ended without a final result.").yellow()
    );
}

fn read_sse_json_events(response: impl Read) -> Vec<Value> {
    let reader = BufReader::new(response);
    let mut events = Vec::new();

    for line in reader.lines().flatten() {
        let Some(json_data) = line.strip_prefix("data: ") else {
            continue;
        };

        if let Ok(json) = serde_json::from_str::<Value>(json_data) {
            events.push(json);
        }
    }

    events
}

fn status_of(event: &Value) -> Option<&str> {
    event.get("status").and_then(|status| status.as_str())
}

fn string_any<'a>(event: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| event.get(*key).and_then(|value| value.as_str()))
}

fn u64_any(event: &Value, keys: &[&str]) -> Option<u64> {
    keys.iter()
        .find_map(|key| event.get(*key).and_then(|value| value.as_u64()))
}

fn f64_any(event: &Value, keys: &[&str]) -> Option<f64> {
    keys.iter()
        .find_map(|key| event.get(*key).and_then(|value| value.as_f64()))
}

fn event_error_message(event: &Value) -> String {
    string_any(event, &["error", "message"])
        .unwrap_or("No error message returned by the server.")
        .to_string()
}

fn print_error_event(event: &Value) {
    let status = status_of(event).unwrap_or("ERROR");
    println!(
        "{}: {}",
        style(status).red().bold(),
        event_error_message(event)
    );

    if let Some(details) = string_any(event, &["details", "stderr", "traceback"]) {
        if !details.trim().is_empty() {
            println!("\n{}", style("Details").bold().underlined());
            println!("{}", details.trim());
        }
    }
}

fn collected_results(events: &[Value], event_status: &str) -> Vec<Value> {
    events
        .iter()
        .filter(|event| status_of(event) == Some(event_status))
        .filter_map(|event| event.get("result").cloned())
        .collect()
}

fn array_field_or_collected(
    event: Option<&Value>,
    field: &str,
    collected: Vec<Value>,
) -> Vec<Value> {
    event
        .and_then(|event| event.get(field))
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or(collected)
}

fn compact_benchmark_results(results: &[Value]) -> Vec<Value> {
    results
        .iter()
        .map(|result| {
            serde_json::json!({
                "test_id": result.get("test_id").cloned().unwrap_or(Value::Null),
                "name": result.get("name").cloned().unwrap_or(Value::Null),
                "gflops": result.get("gflops").cloned().unwrap_or(Value::Null),
                "runtime_ms": result.get("runtime_ms").cloned().unwrap_or(Value::Null),
                "status": result.get("status").cloned().unwrap_or_else(|| Value::String("PASSED".to_string())),
            })
        })
        .collect()
}

pub fn pretty_print_checker_response(response: impl Read, parameters: &Parameters) {
    let events = read_sse_json_events(response);
    let final_event = events.iter().rev().find(|event| {
        matches!(
            status_of(event),
            Some(
                "CHECKED"
                    | "WRONG_ANSWER"
                    | "COMPILE_ERROR"
                    | "RUNTIME_ERROR"
                    | "TIME_LIMIT_EXCEEDED"
                    | "MEMORY_LIMIT_EXCEEDED"
                    | "RATE_LIMIT_EXCEEDED"
                    | "SANDBOX_TIMEOUT"
                    | "SANDBOX_OUTPUT_LIMIT"
                    | "OUTPUT_LIMIT_EXCEEDED"
                    | "ERROR"
            )
        )
    });
    let test_results = array_field_or_collected(
        final_event,
        "test_results",
        collected_results(&events, "TEST_RESULT"),
    );
    let passed_tests = final_event
        .and_then(|event| u64_any(event, &["passed_tests", "passedTests"]))
        .unwrap_or_else(|| {
            test_results
                .iter()
                .filter(|result| status_of(result) == Some("PASSED"))
                .count() as u64
        });
    let total_tests = final_event
        .and_then(|event| u64_any(event, &["total_tests", "totalTests"]))
        .unwrap_or(test_results.len() as u64);
    let status = final_event
        .and_then(status_of)
        .unwrap_or(if events.is_empty() {
            "NO_RESPONSE"
        } else {
            "INCOMPLETE"
        });

    if parameters.get_json_output_flag() {
        let mut output = serde_json::json!({
            "status": status,
            "passed_tests": passed_tests,
            "total_tests": total_tests,
            "test_results": test_results,
        });

        if let Some(final_event) = final_event {
            if let Some(debug_info) = final_event.get("debug_info") {
                output["debug_info"] = debug_info.clone();
            }
            if let Some(message) = string_any(final_event, &["error", "message"]) {
                output["message"] = Value::String(message.to_string());
            }
            if let Some(details) = final_event.get("details") {
                output["details"] = details.clone();
            }
        }

        println!(
            "{}",
            serde_json::to_string_pretty(&output).expect("Failed to serialize checker output")
        );
        return;
    }

    match status {
        "CHECKED" if passed_tests == total_tests => {
            println!("{}", style("✅ CHECKER RESULT: PASSED").green().bold());
            println!("Tests: {}/{} passed", passed_tests, total_tests);
        }
        "CHECKED" | "WRONG_ANSWER" => {
            println!("{}", style("❌ CHECKER RESULT: FAILED").red().bold());
            println!("Tests: {}/{} passed", passed_tests, total_tests);

            if let Some(final_event) = final_event {
                print_json_section("Debug Info", final_event.get("debug_info"));
            }
        }
        "COMPILE_ERROR"
        | "RUNTIME_ERROR"
        | "TIME_LIMIT_EXCEEDED"
        | "MEMORY_LIMIT_EXCEEDED"
        | "RATE_LIMIT_EXCEEDED"
        | "SANDBOX_TIMEOUT"
        | "SANDBOX_OUTPUT_LIMIT"
        | "OUTPUT_LIMIT_EXCEEDED"
        | "ERROR" => {
            if let Some(final_event) = final_event {
                print_error_event(final_event);
            }
        }
        "NO_RESPONSE" => println!("{}", style("Checker returned no response.").red().bold()),
        _ => {
            println!(
                "{}",
                style("Checker stream ended before a final result.").yellow()
            );
            println!("Tests observed: {}/{}", passed_tests, total_tests);
        }
    }

    if !test_results.is_empty() {
        println!("\n{}", style("Test Results").bold().underlined());
        for (index, result) in test_results.iter().enumerate() {
            let name = result
                .get("name")
                .and_then(|value| value.as_str())
                .unwrap_or("Unnamed");
            let result_status = status_of(result).unwrap_or("UNKNOWN");
            let styled_status = if result_status == "PASSED" {
                style(result_status).green().bold()
            } else {
                style(result_status).red().bold()
            };
            println!("{}. {} - {}", index + 1, name, styled_status);
        }
    }
}

pub fn pretty_print_benchmark_response_v2(response: impl Read, parameters: &Parameters) {
    let events = read_sse_json_events(response);
    let final_event = events.iter().rev().find(|event| {
        matches!(
            status_of(event),
            Some(
                "ACCEPTED"
                    | "BENCHMARKED"
                    | "WRONG_ANSWER"
                    | "COMPILE_ERROR"
                    | "RUNTIME_ERROR"
                    | "TIME_LIMIT_EXCEEDED"
                    | "MEMORY_LIMIT_EXCEEDED"
                    | "RATE_LIMIT_EXCEEDED"
                    | "SANDBOX_TIMEOUT"
                    | "SANDBOX_OUTPUT_LIMIT"
                    | "OUTPUT_LIMIT_EXCEEDED"
                    | "ERROR"
            )
        ) || (event.get("avg_gflops").is_some() && event.get("avg_runtime_ms").is_some())
    });
    let benchmark_results = array_field_or_collected(
        final_event,
        "benchmark_results",
        collected_results(&events, "BENCHMARK_RESULT"),
    );
    let avg_gflops = final_event.and_then(|event| f64_any(event, &["avg_gflops", "avgGflops"]));
    let avg_runtime_ms =
        final_event.and_then(|event| f64_any(event, &["avg_runtime_ms", "avgRuntimeMs"]));
    let status = final_event
        .and_then(status_of)
        .unwrap_or(if final_event.is_some() {
            "ACCEPTED"
        } else if events.is_empty() {
            "NO_RESPONSE"
        } else {
            "INCOMPLETE"
        });

    if parameters.get_json_output_flag() {
        let compact_results = compact_benchmark_results(&benchmark_results);
        let mut output = serde_json::json!({
            "status": status,
            "avg_gflops": avg_gflops,
            "avg_runtime_ms": avg_runtime_ms,
            "benchmark_results": compact_results,
        });

        if let Some(final_event) = final_event {
            if let Some(message) = string_any(final_event, &["error", "message"]) {
                output["message"] = Value::String(message.to_string());
            }
            if let Some(details) = final_event.get("details") {
                output["details"] = details.clone();
            }
            if let Some(debug_info) = final_event.get("debug_info") {
                output["debug_info"] = debug_info.clone();
            }
        }

        println!(
            "{}",
            serde_json::to_string_pretty(&output).expect("Failed to serialize benchmark output")
        );
        return;
    }

    match status {
        "ACCEPTED" | "BENCHMARKED" => {
            println!("{}", style("✅ BENCHMARK RESULT: ACCEPTED").green().bold());
            if let Some(avg_runtime_ms) = avg_runtime_ms {
                println!("Average runtime: {:.4} ms", avg_runtime_ms);
            }
            if let Some(avg_gflops) = avg_gflops {
                println!("Average GFLOPS: {:.2}", avg_gflops);
            }
        }
        "WRONG_ANSWER" => {
            println!(
                "{}",
                style("❌ BENCHMARK RESULT: WRONG ANSWER").red().bold()
            );
            if let Some(final_event) = final_event {
                print_json_section("Debug Info", final_event.get("debug_info"));
            }
        }
        "COMPILE_ERROR"
        | "RUNTIME_ERROR"
        | "TIME_LIMIT_EXCEEDED"
        | "MEMORY_LIMIT_EXCEEDED"
        | "RATE_LIMIT_EXCEEDED"
        | "SANDBOX_TIMEOUT"
        | "SANDBOX_OUTPUT_LIMIT"
        | "OUTPUT_LIMIT_EXCEEDED"
        | "ERROR" => {
            if let Some(final_event) = final_event {
                print_error_event(final_event);
            }
        }
        "NO_RESPONSE" => println!("{}", style("Benchmark returned no response.").red().bold()),
        _ => println!(
            "{}",
            style("Benchmark stream ended before a final result.").yellow()
        ),
    }

    if !benchmark_results.is_empty() {
        println!("\n{}", style("Benchmark Results").bold().underlined());
        println!(
            "{:<30} {:>12} {:>16}",
            style("Test Case").bold(),
            style("GFLOPS").bold(),
            style("Runtime (ms)").bold()
        );
        println!("{}", style("─".repeat(62)).dim());

        for result in benchmark_results {
            let name = result
                .get("name")
                .and_then(|value| value.as_str())
                .unwrap_or("Unnamed");
            let gflops = f64_any(&result, &["gflops"]).unwrap_or(0.0);
            let runtime_ms = f64_any(&result, &["runtime_ms", "runtimeMs"]).unwrap_or(0.0);
            println!("{:<30} {:>12.2} {:>16.4}", name, gflops, runtime_ms);
        }
    }
}

pub fn pretty_print_checker_streaming_response(mut response: impl Read) {
    let multi_progress = MultiProgress::new();
    let spinner_style = default_spinner_style();
    let progress_style = default_progress_style();
    let mut compilation_pb = multi_progress.add(ProgressBar::new_spinner());
    compilation_pb.set_style(spinner_style.clone());
    compilation_pb.set_prefix("🔧");
    compilation_pb.enable_steady_tick(Duration::from_millis(80));
    let mut total_tests;
    let mut completed_tests = 0;
    let mut test_progress: Option<ProgressBar> = None;
    let mut test_results: Vec<Value> = Vec::new();
    let mut buffer = [0; 1024];
    let mut data_buffer = String::new();

    while let Ok(size) = response.read(&mut buffer) {
        if size == 0 {
            break;
        }
        let chunk = String::from_utf8_lossy(&buffer[0..size]);
        data_buffer.push_str(&chunk);
        while let Some(pos) = data_buffer.find('\n') {
            let line = data_buffer[..pos].trim().to_string();
            let remaining = data_buffer[pos + 1..].to_string();
            data_buffer = remaining;
            if line.starts_with("data: ") {
                let json_str = &line["data: ".len()..];
                if let Ok(json) = serde_json::from_str::<Value>(json_str) {
                    match json["status"].as_str() {
                        Some("IN_QUEUE") => {
                            compilation_pb.set_message("In queue...".to_string());
                        }
                        Some("COMPILING") => {
                            compilation_pb.set_message("Compiling your code...".to_string());
                        }
                        Some("ERROR") => {
                            compilation_pb.finish_with_message("Error detected!".to_string());
                            compilation_pb.set_prefix("❌");
                            compilation_pb.finish_and_clear();
                            multi_progress.clear().unwrap();
                            println!("\n{}", style("⚠️ ERROR OCCURRED ⚠️").red().bold());
                            println!("{}", style("═".repeat(50)).dim());
                            let error_message = json["error"].as_str().unwrap_or("Unknown error");
                            println!("{}: {}", style("Error").red().bold(), error_message);
                            if let Some(details) = json["details"].as_str() {
                                println!("\n{}", style("Details:").yellow().bold());
                                println!("{}", style("─".repeat(50)).dim());
                                let formatted_details = details
                                    .lines()
                                    .map(|line| {
                                        if line.contains("error:") {
                                            format!("{}", style(line).red())
                                        } else if line.contains("warning:") {
                                            format!("{}", style(line).yellow())
                                        } else if line.contains("^") {
                                            format!("{}", style(line).cyan())
                                        } else {
                                            line.to_string()
                                        }
                                    })
                                    .collect::<Vec<String>>()
                                    .join("\n");
                                println!("{}", formatted_details);
                            }
                            println!("{}", style("═".repeat(50)).dim());
                            println!(
                                "{}",
                                style("Please fix the errors and try again.")
                                    .yellow()
                                    .bold()
                            );
                            return;
                        }
                        Some("CHECKING") => {
                            compilation_pb
                                .finish_with_message("Compilation successful!".to_string());
                            compilation_pb.set_prefix("✅");
                            compilation_pb.finish_and_clear();
                            let running_pb = multi_progress.add(ProgressBar::new_spinner());
                            running_pb.set_style(spinner_style.clone());
                            running_pb.set_prefix("🚀");
                            running_pb.set_message("Running tests...".to_string());
                            running_pb.enable_steady_tick(Duration::from_millis(80));
                            compilation_pb = running_pb;
                        }
                        Some("TEST_RESULT") => {
                            if test_progress.is_none() {
                                if let Some(total) = json["total_tests"].as_u64() {
                                    total_tests = total as usize;
                                    if compilation_pb.is_finished() {
                                        compilation_pb.finish();
                                    }
                                    let progress =
                                        multi_progress.add(ProgressBar::new(total_tests as u64));
                                    progress.set_style(progress_style.clone());
                                    progress.set_prefix("🧪 Tests");
                                    test_progress = Some(progress);
                                }
                            }
                            if let Some(result) = json["result"].as_object() {
                                let test_name = result["name"].as_str().unwrap_or("Unknown test");
                                let status = result["status"].as_str().unwrap_or("UNKNOWN");
                                test_results.push(json["result"].clone());
                                completed_tests += 1;
                                if let Some(ref progress) = test_progress {
                                    let status_symbol =
                                        if status == "PASSED" { "✅" } else { "❌" };
                                    progress.set_position(completed_tests as u64);
                                    progress
                                        .set_message(format!("{} {}", status_symbol, test_name));
                                }
                            }
                        }
                        Some("WRONG_ANSWER") => {
                            if let Some(progress) = test_progress.take() {
                                progress.finish_and_clear();
                            }
                            compilation_pb.finish_and_clear();
                            multi_progress.clear().unwrap();
                            println!("{}", style("❌ Wrong Answer").red().bold());
                            println!("{}", style("═".repeat(65)).dim());
                            println!("Some test cases did not produce the expected output.");
                            println!("{}", style("═".repeat(65)).dim());

                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(json_str) {
                                let debug_info = json["debug_info"].as_object().unwrap();
                                println!("   {}", style("Error Details:").yellow().bold());

                                // Check if there's a message field and display it prominently
                                if let Some(message) = debug_info.get("message") {
                                    if let Some(msg_str) = message.as_str() {
                                        println!(
                                            "   {} {}",
                                            style("→").yellow(),
                                            style(msg_str).red()
                                        );
                                        println!();
                                    }
                                }

                                // Display numerical difference metrics with proper formatting
                                let metrics = [
                                    ("max_difference", "Maximum Difference"),
                                    ("mean_difference", "Mean Difference"),
                                ];

                                for (key, display_name) in metrics.iter() {
                                    if let Some(value) = debug_info.get(*key) {
                                        if value.is_f64() {
                                            let val = value.as_f64().unwrap();
                                            let formatted_val = format!("{:.6e}", val);
                                            println!(
                                                "   {} {}: {}",
                                                style("■").cyan(),
                                                style(*display_name).cyan(),
                                                formatted_val
                                            );
                                        }
                                    }
                                }

                                if let Some(sample_diffs) = debug_info.get("sample_differences") {
                                    if let Some(diffs_map) = sample_diffs.as_object() {
                                        if !diffs_map.is_empty() {
                                            println!(
                                                "   {} {}:",
                                                style("■").cyan(),
                                                style("Sample Differences").cyan()
                                            );

                                            let max_samples = 5.min(diffs_map.len());
                                            for (i, (coord, vals)) in
                                                diffs_map.iter().take(max_samples).enumerate()
                                            {
                                                if let (Some(actual), Some(diff), Some(expected)) = (
                                                    vals.get("actual").and_then(|v| v.as_f64()),
                                                    vals.get("diff").and_then(|v| v.as_f64()),
                                                    vals.get("expected").and_then(|v| v.as_f64()),
                                                ) {
                                                    println!(
                        "     - Sample {}: Coord {} => actual: {:.6e}, expected: {:.6e}, diff: {:.6e}",
                        i + 1,
                        coord,
                        actual,
                        expected,
                        diff
                    );
                                                }
                                            }

                                            if diffs_map.len() > max_samples {
                                                println!(
                                                    "     - {} more differences...",
                                                    diffs_map.len() - max_samples
                                                );
                                            }
                                        }
                                    }
                                }

                                // Display any other fields that might be present
                                for (key, value) in debug_info {
                                    if key != "message"
                                        && key != "max_difference"
                                        && key != "mean_difference"
                                        && key != "sample_differences"
                                    {
                                        let formatted_value = if value.is_f64() {
                                            format!("{:.6}", value.as_f64().unwrap())
                                        } else {
                                            value.to_string().replace("\"", "")
                                        };

                                        println!(
                                            "   {} {}: {}",
                                            style("■").cyan(),
                                            style(key).cyan(),
                                            formatted_value
                                        );
                                    }
                                }
                            }
                        }
                        Some("CHECKED") => {
                            if let Some(progress) = test_progress.take() {
                                progress.finish_and_clear();
                            }
                            compilation_pb.finish_and_clear();
                            compilation_pb.set_prefix("✅");

                            let passed_tests = json["passedTests"].as_u64().unwrap_or(0);
                            let total_tests = json["totalTests"].as_u64().unwrap_or(0);
                            let passed = passed_tests == total_tests;

                            multi_progress.clear().unwrap();
                            std::thread::sleep(Duration::from_millis(500));

                            let header = if passed {
                                style("✨ ALL TESTS PASSED! ✨").green().bold()
                            } else {
                                style("⚠️ TESTS FAILED ⚠️").red().bold()
                            };

                            println!("{}", header);
                            println!("{}", style("═".repeat(65)).dim());
                            println!("Tests: {}/{} passed", passed_tests, total_tests);
                            println!("{}", style("═".repeat(65)).dim());

                            println!("\n{}", style("Test Results:").bold().underlined());

                            if let Some(results) = json["test_results"].as_array() {
                                for result in results.iter() {
                                    let test_id = result["test_id"].as_u64().unwrap_or(0);
                                    let test_name =
                                        result["name"].as_str().unwrap_or("Unknown test");
                                    let status = result["status"].as_str().unwrap_or("UNKNOWN");
                                    let status_style = if status == "PASSED" {
                                        style(status).green().bold()
                                    } else {
                                        style(status).red().bold()
                                    };
                                    println!("{}. {} - {}", test_id, test_name, status_style);

                                    if status == "FAILED" && result.get("debug_info").is_some() {
                                        if let Some(debug_info) = result["debug_info"].as_object() {
                                            println!(
                                                "   {}",
                                                style("Error Details:").yellow().bold()
                                            );

                                            // Check if there's a message field and display it prominently
                                            if let Some(message) = debug_info.get("message") {
                                                if let Some(msg_str) = message.as_str() {
                                                    println!(
                                                        "   {} {}",
                                                        style("→").yellow(),
                                                        style(msg_str).red()
                                                    );
                                                    println!();
                                                }
                                            }

                                            // Display numerical difference metrics with proper formatting
                                            let metrics = [
                                                ("max_difference", "Maximum Difference"),
                                                ("mean_difference", "Mean Difference"),
                                            ];

                                            for (key, display_name) in metrics.iter() {
                                                if let Some(value) = debug_info.get(*key) {
                                                    if value.is_f64() {
                                                        let val = value.as_f64().unwrap();
                                                        let formatted_val = format!("{:.6e}", val);
                                                        println!(
                                                            "   {} {}: {}",
                                                            style("■").cyan(),
                                                            style(*display_name).cyan(),
                                                            formatted_val
                                                        );
                                                    }
                                                }
                                            }

                                            // Handle sample differences in a more compact format
                                            if let Some(sample_diffs) =
                                                debug_info.get("sample_differences")
                                            {
                                                if let Some(diffs_array) = sample_diffs.as_array() {
                                                    if !diffs_array.is_empty() {
                                                        println!(
                                                            "   {} {}:",
                                                            style("■").cyan(),
                                                            style("Sample Differences").cyan()
                                                        );

                                                        // Only show up to 5 differences to avoid flooding the console
                                                        let max_samples = 5.min(diffs_array.len());
                                                        for i in 0..max_samples {
                                                            if let Some(diff) =
                                                                diffs_array[i].as_f64()
                                                            {
                                                                println!(
                                                                    "     - Sample {}: {:.6e}",
                                                                    i + 1,
                                                                    diff
                                                                );
                                                            }
                                                        }

                                                        if diffs_array.len() > max_samples {
                                                            println!(
                                                                "     - {} more differences...",
                                                                diffs_array.len() - max_samples
                                                            );
                                                        }
                                                    }
                                                }
                                            }

                                            // Display any other fields that might be present
                                            for (key, value) in debug_info {
                                                if key != "message"
                                                    && key != "max_difference"
                                                    && key != "mean_difference"
                                                    && key != "sample_differences"
                                                {
                                                    let formatted_value = if value.is_f64() {
                                                        format!("{:.6}", value.as_f64().unwrap())
                                                    } else {
                                                        value.to_string().replace("\"", "")
                                                    };

                                                    println!(
                                                        "   {} {}: {}",
                                                        style("■").cyan(),
                                                        style(key).cyan(),
                                                        formatted_value
                                                    );
                                                }
                                            }
                                            println!();
                                        }
                                    }
                                }
                            } else {
                                // Fallback to using the collected test results if test_results not in JSON
                                for (i, result) in test_results.iter().enumerate() {
                                    let test_name =
                                        result["name"].as_str().unwrap_or("Unknown test");
                                    let status = result["status"].as_str().unwrap_or("UNKNOWN");
                                    let status_style = if status == "PASSED" {
                                        style(status).green().bold()
                                    } else {
                                        style(status).red().bold()
                                    };
                                    println!("{}. {} - {}", i + 1, test_name, status_style);
                                }
                            }

                            println!("\n{}", style("═".repeat(65)).dim());
                        }
                        _ => {
                            // Uncomment for debugging
                            // println!("{}", line);
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct TestResultInner {
    status: String, // PASSED / FAILED
}

#[derive(Debug, Deserialize)]
struct TestResultData {
    result: Option<TestResultInner>,
}

#[derive(Debug, Deserialize)]
struct CheckedData {
    passed_tests: Option<u32>,
    total_tests: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct AcceptedData {
    avg_runtime_ms: Option<f64>,
    avg_gflops: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct ErrorData {
    error: Option<String>,
    message: Option<String>,
}

pub fn pretty_print_auth() {
    println!("\n🎉  Authentication successful!");
    println!("Your token has been securely saved to ~/.tensara/auth.json");
    println!("You're ready to run commands like `tensara submit` or `tensara benchmark`.\n");
}

pub fn pretty_print_submit_response(response: impl Read) {
    let multi_progress = MultiProgress::new();

    let spinner = multi_progress.add(ProgressBar::new_spinner());
    spinner.set_style(default_spinner_style());
    spinner.set_message("🚀 Submitting...");
    spinner.enable_steady_tick(Duration::from_millis(80));

    let mut progress_bar: Option<ProgressBar> = None;
    let reader = BufReader::new(response);
    let mut current_event: Option<String> = None;

    let mut passed_tests: u64 = 0;
    let mut total_tests: u64 = 0;
    let mut total_benchmarks: u64 = 0;
    let mut benchmark_results = vec![];
    let mut completed = 0;

    for line in reader.lines().flatten() {
        spinner.tick();

        if line.starts_with("event: ") {
            current_event = Some(line[7..].trim().to_string());
            continue;
        }

        if !line.starts_with("data: ") {
            continue;
        }

        let json_data = &line[6..];

        match current_event.as_deref() {
            Some("IN_QUEUE") => {
                spinner.set_prefix("🧘");
                spinner.set_message("In queue...");
            }

            Some("COMPILING") => {
                spinner.set_prefix("🔧");
                spinner.set_message("Compiling...");
            }

            Some("CHECKING") => {
                spinner.set_prefix("🔍");
                spinner.set_message("Checking...");
            }

            Some("TEST_RESULT") => {
                if progress_bar.is_none() {
                    let pb = multi_progress.add(ProgressBar::new(0));
                    pb.set_style(default_progress_style());
                    pb.set_prefix("📊 Tests");
                    progress_bar = Some(pb);
                }

                if total_tests == 0 {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_data) {
                        total_tests = value
                            .get("total_tests")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        if let Some(ref pb) = progress_bar {
                            pb.set_length(total_tests);
                        }
                    }
                }

                if let Ok(data) = serde_json::from_str::<TestResultData>(json_data) {
                    if let Some(result) = data.result {
                        if result.status == "PASSED" {
                            passed_tests += 1;
                        }
                        if let Some(ref pb) = progress_bar {
                            pb.set_position(passed_tests);
                            pb.set_message(format!("✅ {passed_tests} passed"));
                        }
                    }
                }
            }

            Some("CHECKED") => {
                if let Ok(data) = serde_json::from_str::<CheckedData>(json_data) {
                    if let (Some(p), Some(t)) = (data.passed_tests, data.total_tests) {
                        passed_tests = p as u64;
                        total_tests = t as u64;
                        if let Some(ref pb) = progress_bar {
                            pb.set_length(total_tests);
                            pb.set_position(passed_tests);
                            pb.set_message(format!("✅ {passed_tests} passed"));
                        }
                    }
                }
                if let Some(pb) = progress_bar.take() {
                    pb.finish_and_clear();
                }
                passed_tests = 0;
                total_tests = 0;
            }

            Some("BENCHMARKING") => {
                spinner.set_prefix("⚡");
                spinner.set_message("Running benchmarks...");
            }

            Some("BENCHMARK_RESULT") => {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(json_data) {
                    if total_benchmarks == 0 {
                        total_benchmarks = json["total_tests"].as_u64().unwrap_or(0);
                        let pb = multi_progress.add(ProgressBar::new(total_benchmarks));
                        pb.set_style(default_progress_style());
                        pb.set_prefix("📊 Benchmarks");
                        progress_bar = Some(pb);
                    }

                    if let Some(result) = json.get("result") {
                        benchmark_results.push(result.clone());
                        completed += 1;

                        if let Some(pb) = &progress_bar {
                            let name = result["name"].as_str().unwrap_or("Unnamed");
                            let gflops = result["gflops"].as_f64().unwrap_or(0.0);
                            let runtime = result["runtime_ms"].as_f64().unwrap_or(0.0);

                            pb.set_position(completed);
                            pb.set_message(format!(
                                "{}: {:.2} GFLOPS ({:.2} ms)",
                                name, gflops, runtime
                            ));
                        }
                    }
                }
            }

            Some("ACCEPTED") => {
                if let Some(pb) = progress_bar.take() {
                    pb.finish_and_clear();
                }
                spinner.finish_and_clear();
                multi_progress.clear().unwrap();

                println!(
                    "{}",
                    style("🎯 SUBMISSION RESULT: ACCEPTED ").green().bold()
                );

                if let Ok(data) = serde_json::from_str::<AcceptedData>(json_data) {
                    let avg_rt = data.avg_runtime_ms.unwrap_or(0.0);
                    let avg_gflops = data.avg_gflops.unwrap_or(0.0);
                    println!(
                        "⏱ Avg runtime: \x1b[32m{:.2} ms\x1b[0m\n🚀 Avg gflops: \x1b[34m{:.2}\x1b[0m",
                        avg_rt, avg_gflops
                    );
                }

                if let Some(pb) = progress_bar.take() {
                    pb.finish();
                }
                spinner.finish();
                multi_progress.clear().unwrap();
                thread::sleep(Duration::from_millis(100));

                if let Ok(json) = serde_json::from_str::<serde_json::Value>(json_data) {
                    let empty = vec![];
                    let results = json["benchmark_results"].as_array().unwrap_or(&empty);

                    println!("\n{}", style("Detailed Results:").bold().underlined());
                    println!(
                        "{:<30} {:>10} {:>15} {:>15}",
                        style("Test Case").bold(),
                        style("GFLOPS").bold(),
                        style("Runtime (ms)").bold(),
                        style("Status").bold()
                    );
                    println!("{}", style("─".repeat(75)).dim());

                    for (i, result) in results.iter().enumerate() {
                        let name = result["name"].as_str().unwrap_or("Unnamed");
                        let gflops = result["gflops"].as_f64().unwrap_or(0.0);
                        let runtime = result["runtime_ms"].as_f64().unwrap_or(0.0);

                        println!(
                            "{} {:<3} {:<24} {:>10.2} {:>15.4} {:>15}",
                            style("✓").green().bold(),
                            i + 1,
                            name,
                            gflops,
                            runtime,
                            "PASSED"
                        );
                    }
                }

                break;
            }

            Some("WRONG_ANSWER") => {
                if let Some(pb) = progress_bar.take() {
                    pb.finish_and_clear();
                }
                spinner.abandon_with_message("❌ Wrong Answer");
                spinner.finish();

                if let Ok(json) = serde_json::from_str::<serde_json::Value>(json_data) {
                    let debug_info = json["debug_info"].as_object().unwrap();
                    println!("   {}", style("Error Details:").yellow().bold());

                    // Check if there's a message field and display it prominently
                    if let Some(message) = debug_info.get("message") {
                        if let Some(msg_str) = message.as_str() {
                            println!("   {} {}", style("→").yellow(), style(msg_str).red());
                            println!();
                        }
                    }

                    // Display numerical difference metrics with proper formatting
                    let metrics = [
                        ("max_difference", "Maximum Difference"),
                        ("mean_difference", "Mean Difference"),
                    ];

                    for (key, display_name) in metrics.iter() {
                        if let Some(value) = debug_info.get(*key) {
                            if value.is_f64() {
                                let val = value.as_f64().unwrap();
                                let formatted_val = format!("{:.6e}", val);
                                println!(
                                    "   {} {}: {}",
                                    style("■").cyan(),
                                    style(*display_name).cyan(),
                                    formatted_val
                                );
                            }
                        }
                    }

                    if let Some(sample_diffs) = debug_info.get("sample_differences") {
                        if let Some(diffs_map) = sample_diffs.as_object() {
                            if !diffs_map.is_empty() {
                                println!(
                                    "   {} {}:",
                                    style("■").cyan(),
                                    style("Sample Differences").cyan()
                                );

                                let max_samples = 5.min(diffs_map.len());
                                for (i, (coord, vals)) in
                                    diffs_map.iter().take(max_samples).enumerate()
                                {
                                    if let (Some(actual), Some(diff), Some(expected)) = (
                                        vals.get("actual").and_then(|v| v.as_f64()),
                                        vals.get("diff").and_then(|v| v.as_f64()),
                                        vals.get("expected").and_then(|v| v.as_f64()),
                                    ) {
                                        println!(
                        "     - Sample {}: Coord {} => actual: {:.6e}, expected: {:.6e}, diff: {:.6e}",
                        i + 1,
                        coord,
                        actual,
                        expected,
                        diff
                    );
                                    }
                                }

                                if diffs_map.len() > max_samples {
                                    println!(
                                        "     - {} more differences...",
                                        diffs_map.len() - max_samples
                                    );
                                }
                            }
                        }
                    }

                    // Display any other fields that might be present
                    for (key, value) in debug_info {
                        if key != "message"
                            && key != "max_difference"
                            && key != "mean_difference"
                            && key != "sample_differences"
                        {
                            let formatted_value = if value.is_f64() {
                                format!("{:.6}", value.as_f64().unwrap())
                            } else {
                                value.to_string().replace("\"", "")
                            };

                            println!(
                                "   {} {}: {}",
                                style("■").cyan(),
                                style(key).cyan(),
                                formatted_value
                            );
                        }
                    }
                }

                break;
            }

            Some("ERROR") => {
                if let Some(pb) = progress_bar.take() {
                    pb.finish_and_clear();
                }
                if let Ok(data) = serde_json::from_str::<ErrorData>(json_data) {
                    let msg = data
                        .error
                        .or(data.message)
                        .unwrap_or_else(|| "Unknown error".to_string());
                    spinner.abandon_with_message(format!("❌ Error: {msg}"));
                } else {
                    spinner.abandon_with_message("❌ Unknown error");
                }
                break;
            }

            Some("COMPILE_ERROR") => {
                if let Some(pb) = progress_bar.take() {
                    pb.finish_and_clear();
                }
                spinner.abandon_with_message("❌ Compile error");

                if let Ok(json) = serde_json::from_str::<serde_json::Value>(json_data) {
                    let details = json["details"].as_str().unwrap_or("Unknown error");
                    println!("{}", style(details).red().bold());
                }

                break;
            }

            Some(_) => {}
            None => {}
        }
    }
}

pub fn pretty_print_benchmark_response(mut response: impl Read) {
    let multi_progress = MultiProgress::new();
    let progress_style = default_progress_style();
    let spinner = multi_progress.add(ProgressBar::new_spinner());
    spinner.set_style(default_spinner_style());
    spinner.set_prefix("🔧");
    spinner.enable_steady_tick(Duration::from_millis(80));

    let mut total_benchmarks = 0;
    let mut completed = 0;
    let mut progress_bar: Option<ProgressBar> = None;
    let mut buffer = [0; 1024];
    let mut data_buffer = String::new();
    let mut benchmark_results = vec![];

    while let Ok(size) = response.read(&mut buffer) {
        if size == 0 {
            break;
        }

        let chunk = String::from_utf8_lossy(&buffer[..size]);
        data_buffer.push_str(&chunk);

        while let Some(pos) = data_buffer.find('\n') {
            let line = data_buffer[..pos].trim().to_string();
            data_buffer = data_buffer[pos + 1..].to_string();

            if line.starts_with("data: ") {
                let json_str = &line[6..];
                if let Ok(json) = serde_json::from_str::<Value>(json_str) {
                    if let Some(status) = json.get("status").and_then(|s| s.as_str()) {
                        match status {
                            "COMPILING" => spinner.set_message("Compiling your code..."),
                            "SANITY_CHECK_PASSED" => {
                                spinner.set_prefix("🔍");
                                spinner.set_message("Sanity check passed...");
                            }
                            "BENCHMARKING" => {
                                spinner.set_prefix("⚡");
                                spinner.set_message("Running benchmarks...");
                            }
                            "BENCHMARK_RESULT" => {
                                if total_benchmarks == 0 {
                                    total_benchmarks = json["total_tests"].as_u64().unwrap_or(0);
                                    let pb = multi_progress.add(ProgressBar::new(total_benchmarks));
                                    pb.set_style(progress_style.clone());
                                    pb.set_prefix("📊 Benchmarks");
                                    progress_bar = Some(pb);
                                }

                                if let Some(result) = json.get("result") {
                                    benchmark_results.push(result.clone());
                                    completed += 1;

                                    if let Some(pb) = &progress_bar {
                                        let name = result["name"].as_str().unwrap_or("Unnamed");
                                        let gflops = result["gflops"].as_f64().unwrap_or(0.0);
                                        let runtime = result["runtime_ms"].as_f64().unwrap_or(0.0);

                                        pb.set_position(completed);
                                        pb.set_message(format!(
                                            "{}: {:.2} GFLOPS ({:.2} ms)",
                                            name, gflops, runtime
                                        ));
                                    }
                                }
                            }
                            _ => {}
                        }
                    } else if json.get("avg_gflops").is_some()
                        && json.get("avg_runtime_ms").is_some()
                        && json.get("benchmark_results").is_some()
                    {
                        if let Some(pb) = progress_bar.take() {
                            pb.finish();
                        }
                        spinner.finish();
                        multi_progress.clear().unwrap();
                        thread::sleep(Duration::from_millis(100));

                        let avg_gflops = json["avg_gflops"].as_f64().unwrap_or(0.0);
                        let avg_runtime = json["avg_runtime_ms"].as_f64().unwrap_or(0.0);
                        let empty = vec![];
                        let results = json["benchmark_results"].as_array().unwrap_or(&empty);

                        println!("{}", style("BENCHMARK RESULTS ").green().bold());
                        println!("{}", style("═".repeat(65)).dim());
                        println!(
                            "{:<25} {:>15}",
                            style("Metric").bold(),
                            style("Value").bold()
                        );
                        println!("{}", style("─".repeat(65)).dim());
                        println!("{:<25} {:>15}", "Total Benchmarks:", results.len());
                        println!("{:<25} {:>15.2}", "Average GFLOPS:", avg_gflops);
                        println!("{:<25} {:>15.2} ms", "Average Runtime:", avg_runtime);
                        println!("{}", style("═".repeat(65)).dim());

                        println!("\n{}", style("Detailed Results:").bold().underlined());
                        println!(
                            "{:<30} {:>10} {:>15} {:>15}",
                            style("Test Case").bold(),
                            style("GFLOPS").bold(),
                            style("Runtime (ms)").bold(),
                            style("Status").bold()
                        );
                        println!("{}", style("─".repeat(75)).dim());

                        for (i, result) in results.iter().enumerate() {
                            let name = result["name"].as_str().unwrap_or("Unnamed");
                            let gflops = result["gflops"].as_f64().unwrap_or(0.0);
                            let runtime = result["runtime_ms"].as_f64().unwrap_or(0.0);

                            println!(
                                "{} {:<3} {:<24} {:>10.2} {:>15.4} {:>15}",
                                style("✓").green().bold(),
                                i + 1,
                                name,
                                gflops,
                                runtime,
                                "PASSED"
                            );
                        }

                        println!(
                            "\n{}",
                            style("🏁 Benchmark completed successfully! 🏁")
                                .green()
                                .bold()
                        );
                    }
                }
            }
        }
    }
}

pub fn print_parse_error(error: &clap::Error) {
    match error.kind() {
        clap::error::ErrorKind::InvalidValue => {
            let error_message = error.to_string();

            if error_message.contains("PROBLEM_NAME") {
                print_invalid_problem_error(&error_message);
            } else if error_message.contains("GPU_TYPE") {
                print_invalid_gpu_error(&error_message);
            } else if error_message.contains("SOLUTION_FILE") {
                print_invalid_file_error();
            } else if error_message.contains("NOT_SUPPORTED") {
                print_unsupported_file_error();
            } else {
                print_generic_error(&error_message);
            }
        }
        clap::error::ErrorKind::MissingRequiredArgument => {
            print_missing_arg_error(&error.to_string());
        }
        clap::error::ErrorKind::DisplayHelp => {
            print_help(&error.to_string());
        }
        _ => {
            print_generic_error(&error.to_string());
        }
    }
}

fn print_invalid_problem_error(error_message: &str) {
    let problem_value = extract_value_from_error(error_message);
    println!("\n{}", style("⚠️ INVALID PROBLEM NAME ⚠️").red().bold());

    if let Some(value) = problem_value {
        println!(
            "\n{}: '{}'",
            style("Invalid problem name").yellow().bold(),
            value
        );
    }

    println!(
        "\n{}",
        style("See https://tensara.org/problems for details").yellow()
    );
    println!("{}", style("═".repeat(60)).dim());
}

fn print_invalid_gpu_error(error_message: &str) {
    let gpu_value = extract_value_from_error(error_message);

    println!("\n{}", style("⚠️ INVALID GPU TYPE ⚠️").red().bold());
    println!("{}", style("═".repeat(50)).dim());

    if let Some(value) = gpu_value {
        println!(
            "\n{}: '{}'",
            style("Invalid GPU type").yellow().bold(),
            value
        );
    }

    println!("\n{}", style("Supported GPU types:").green().bold());
    println!("{}", style("─".repeat(50)).dim());

    let gpus = [
        ("T4", "NVIDIA Tesla T4"),
        ("A100", "NVIDIA A100"),
        ("A100_80GB", "NVIDIA A100 80GB"),
        ("H100", "NVIDIA H100"),
        ("L4", "NVIDIA L4"),
        ("L40s", "NVIDIA L40S"),
    ];

    for (name, desc) in gpus {
        println!("  • {} - {}", style(name).cyan().bold(), desc);
    }

    println!("{}", style("═".repeat(50)).dim());
}

fn print_invalid_file_error() {
    println!("\n{}", style("⚠️ INVALID SOLUTION FILE ⚠️").red().bold());
    println!("{}", style("═".repeat(60)).dim());

    println!("\n{}", style("Requirements:").green().bold());
    println!("{}", style("─".repeat(60)).dim());
    println!("  • File must exist");
    println!("  • File must be either a .cu (CUDA) .py (Python) or .mojo (Mojo) file");
    println!("  • File must be readable");

    println!("\n{}", style("Example:").yellow().bright().bold());
    println!("  tensara checker -p relu -s ./my_solution.cu");

    println!("{}", style("═".repeat(60)).dim());
}

fn print_missing_arg_error(error_message: &str) {
    println!(
        "\n{}",
        style("⚠️ MISSING REQUIRED ARGUMENT ⚠️").red().bold()
    );
    println!("{}", style("═".repeat(60)).dim());
    println!("{}", style(error_message).red());
}

fn print_generic_error(error_message: &str) {
    println!("\n{}", style("⚠️ COMMAND ERROR ⚠️").red().bold());
    println!("{}", error_message);

    println!(
        "\n{}",
        style("Try running with --help for more information").yellow()
    );
    println!("{}", style("═".repeat(60)).dim());
}

pub fn print_file_error(file_path: &str, error_message: &str) {
    println!("\n{}", style("⚠️ FILE ERROR ⚠️").red().bold());
    println!("{}", style("═".repeat(60)).dim());
    println!("{}: {}", style("Error").red().bold(), error_message);
    println!("{}: {}", style("File").yellow().bold(), file_path);

    println!("\n{}", style("Make sure:").green().bold());
    println!("  • The file exists");
    println!("  • You have permission to read the file");
    println!("  • The file path is correct");

    println!("{}", style("═".repeat(60)).dim());
}

pub fn print_auth_error() {
    println!("\n{}", style("⚠️ AUTHENTICATION ERROR ⚠️").red().bold());
    println!("{}", style("═".repeat(60)).dim());
    println!(
        "{}",
        style("Authentication failed. Please run `tensara auth` to authenticate.")
            .yellow()
            .bold()
    );
    println!(
        "{}",
        style("Check the status of your API keys here:")
            .yellow()
            .bold()
    );
    println!("{}", style("https://tensara.org/cli").yellow());
    println!("{}", style("═".repeat(60)).dim());
}

pub fn print_request_error(error_message: &str) {
    println!("\n{}", style("⚠️ REQUEST FAILED ⚠️").red().bold());
    println!("{}", style("═".repeat(60)).dim());
    println!(
        "{}: {}",
        style("Reason").red().bold(),
        style(error_message).yellow()
    );
    println!(
        "{}",
        style("Check your network connection and Tensara API base URL.")
            .yellow()
            .bold()
    );
    println!("{}", style("═".repeat(60)).dim());
}

pub fn print_http_error(error: &HttpError) {
    println!("\n{}", style("⚠️ SERVER ERROR ⚠️").red().bold());
    println!("{}", style("═".repeat(72)).dim());
    println!(
        "{}: {}",
        style("Status").red().bold(),
        style(&error.status_text).yellow()
    );
    println!(
        "{}: {}",
        style("Endpoint").cyan().bold(),
        style(&error.endpoint).dim()
    );

    if let Some(content_type) = &error.content_type {
        println!(
            "{}: {}",
            style("Content-Type").cyan().bold(),
            style(content_type).dim()
        );
    }

    let summary = error
        .error
        .as_deref()
        .or(error.message.as_deref())
        .unwrap_or("Request failed");
    println!(
        "{}: {}",
        style("Message").yellow().bold(),
        style(summary).red()
    );

    if let Some(details) = error.details.as_deref() {
        println!("\n{}", style("Details:").yellow().bold());
        println!("{}", details);
    } else if !error.raw_body.trim().is_empty()
        && error.raw_body.trim() != "null"
        && error.raw_body.trim() != "{}"
    {
        println!("\n{}", style("Response Body:").yellow().bold());
        println!("{}", error.raw_body.trim());
    }

    match error.status_code {
        404 => println!(
            "\n{}",
            style(
                "The Tensara endpoint was not found. Check that your CLI is pointing at the right backend."
            )
            .yellow()
            .bold()
        ),
        429 => println!(
            "\n{}",
            style("Rate limit exceeded. Wait a bit before retrying.")
                .yellow()
                .bold()
        ),
        500..=599 => println!(
            "\n{}",
            style("The Tensara backend failed while handling your request.")
                .yellow()
                .bold()
        ),
        _ => {}
    }

    println!("{}", style("═".repeat(72)).dim());
}

fn extract_value_from_error(error_message: &str) -> Option<String> {
    let parts: Vec<&str> = error_message.split(':').collect();
    if parts.len() >= 3 {
        return Some(parts[2].trim().to_string());
    }
    None
}

pub fn print_welcome_message() {
    println!(
        "\n{}",
        style(format!("✨ Welcome to Tensara ✨",)).blue().bold()
    );
    println!("{}", style("═".repeat(60)).dim());

    println!("\n{}", style("About:").blue().bold());
    println!("A CLI tool for submitting and benchmarking solutions to GPU programming problems.");
    println!(
        "Find available problems at: {}",
        style("https://tensara.org/problems").yellow()
    );

    println!("\n{}", style("Available Commands:").blue().bold());
    println!("{}", style("─".repeat(60)).dim());
    println!(
        "  • {} - {}",
        style("auth").green().bold(),
        "Authenticate with the Tensara API to submit solutions"
    );
    println!(
        "  • {} - {}",
        style("problems").green().bold(),
        "List all available problems"
    );
    println!(
        "  • {} - {}",
        style("problem").green().bold(),
        "Show a problem description and PyTorch reference solution"
    );
    println!(
        "  • {} - {}",
        style("init").green().bold(),
        "Download a problem with starter code and description"
    );
    println!(
        "  • {} - {}",
        style("checker").green().bold(),
        "Run and check your solution against the reference output"
    );
    println!(
        "  • {} - {}",
        style("benchmark").green().bold(),
        "Benchmark your solution and get performance metrics"
    );
    println!(
        "  • {} - {}",
        style("submit").green().bold(),
        "Submit a solution to Tensara"
    );

    println!("\n{}", style("Example Usage:").blue().bold());
    println!("{}", style("─".repeat(60)).dim());
    println!(
        "  • {}",
        style("tensara checker -p conv-1d -s solution.cu")
            .yellow()
            .bright()
    );
    println!(
        "  • {}",
        style("tensara benchmark -g A100 -p matrix-multiplication -s solution.py")
            .yellow()
            .bright()
    );

    println!(
        "  • {}",
        style("tensara benchmark -g A100 --problem matrix-vector --solution ./solution.py")
            .yellow()
            .bright()
    );
    println!(
        "  • {}",
        style("tensara submit -p relu -s solution.cu")
            .yellow()
            .bright()
    );

    println!("\n{}", style("For Help:").blue().bold());
    println!("{}", style("─".repeat(60)).dim());
    println!("  • {}", style("tensara --help").yellow());
    println!("  • {}", style("tensara submit --help").yellow());
    println!("  • {}", style("tensara checker --help").yellow());
    println!("  • {}", style("tensara benchmark --help").yellow());
    println!("  • {}", style("tensara problems --help").yellow());
    println!("  • {}", style("tensara auth --help").yellow());
    println!("  • {}", style("tensara init --help").yellow());

    println!("{}", style("═".repeat(60)).dim());
}

fn print_help(error_message: &str) {
    println!("{}", style("═".repeat(60)).dim());
    println!("{}", error_message);
    println!("{}", style("═".repeat(60)).dim());
}

fn print_unsupported_file_error() {
    println!("{}", style("═".repeat(60)).dim());
    println!(
        "{}",
        style("We are actively working on enabling Triton support! Please check tensara.org for updates.")
            .green()
            .bold()
    );
    println!("{}", style("═".repeat(60)).dim());
}

fn default_spinner_style() -> ProgressStyle {
    ProgressStyle::default_spinner()
        .template("{spinner:.green} {prefix:.bold.dim} {wide_msg}")
        .unwrap()
        .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
}

fn default_progress_style() -> ProgressStyle {
    ProgressStyle::default_bar()
        .template("{prefix:.bold.blue} [{bar:40.blue/cyan}] {pos}/{len} {msg}")
        .unwrap()
        .progress_chars("█▓▒░  ")
}
