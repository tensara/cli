use crate::trpc::get_all_problems;
use std::fs;
use std::fs::File;

fn write_problems_to_disk(problems_path: &std::path::Path) {
    println!("Fetching problems...");
    let problems = get_all_problems().unwrap_or_else(|error| {
        eprintln!("Failed to fetch problems: {}", error);
        std::process::exit(1);
    });

    let file = File::create(problems_path).expect("Failed to create problems file");
    serde_json::to_writer_pretty(file, &problems).expect("Failed to write problems");

    println!("Pulled problems and updated problems.json successfully.");
}

pub fn refresh_problems() {
    let problems_path = dirs::home_dir()
        .expect("Could not find home directory")
        .join(".tensara")
        .join("problems.json");

    write_problems_to_disk(&problems_path);
}

pub fn is_valid_problem_slug(slug: &str) -> bool {
    fn check(slug: &str, contents: &str) -> bool {
        if let Ok(problems) = serde_json::from_str::<Vec<serde_json::Value>>(contents) {
            return problems.iter().any(|problem| {
                problem
                    .get("slug")
                    .and_then(|s| s.as_str())
                    .map(|s| s.eq_ignore_ascii_case(slug))
                    .unwrap_or(false)
            });
        }
        false
    }

    let problems_path = dirs::home_dir()
        .expect("Could not find home directory")
        .join(".tensara")
        .join("problems.json");

    if let Ok(contents) = fs::read_to_string(&problems_path) {
        if check(slug, &contents) {
            return true;
        }
    }

    println!("Problem not found or file missing — refreshing problems.json...");
    refresh_problems();

    if let Ok(contents) = fs::read_to_string(&problems_path) {
        return check(slug, &contents);
    }

    false
}
