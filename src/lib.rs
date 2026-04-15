pub mod api;
pub mod auth;
pub mod client;
pub mod init;
pub mod parser;
pub mod pretty;
pub mod problems;
pub mod trpc;
use clap::ArgMatches;

pub enum CommandType {
    Checker,
    Benchmark,
    Submit,
    Sample,
    Problems,
    Problem,
    Auth,
    Init,
    None,
}

pub struct Parameters {
    command_type: CommandType,

    // Common fields
    command_name: String,

    // Problem-related fields
    problem_slug: Option<String>,
    code: Option<String>,
    dtype: Option<String>,
    language: Option<String>,
    gpu_type: Option<String>,

    // Problems command fields
    fields: Option<Vec<String>>,
    sort_by: Option<String>,
    description_only: bool,
    reference_only: bool,
    json_output: bool,

    // Auth command fields
    token: Option<String>,

    // Init command fields
    directory: Option<String>,
    all_flag: bool,
}

impl Parameters {
    pub fn new() -> Self {
        let command_matches = match parser::parse_args(None) {
            Ok(matches) => matches,
            Err(e) => match e.kind() {
                clap::error::ErrorKind::DisplayHelp => {
                    println!("{}", e.to_string());
                    std::process::exit(0);
                }
                clap::error::ErrorKind::DisplayVersion => {
                    println!("{}", e.to_string());
                    std::process::exit(0);
                }
                _ => {
                    pretty::print_parse_error(&e);
                    std::process::exit(1);
                }
            },
        };

        match command_matches.subcommand_name() {
            Some("checker") => Self::from_subcommand(
                CommandType::Checker,
                "checker",
                parser::get_checker_matches(&command_matches),
            ),
            Some("benchmark") => Self::from_subcommand(
                CommandType::Benchmark,
                "benchmark",
                parser::get_benchmark_matches(&command_matches),
            ),
            Some("submit") => Self::from_subcommand(
                CommandType::Submit,
                "submit",
                parser::get_submit_matches(&command_matches),
            ),
            Some("sample") => Self::from_subcommand(
                CommandType::Sample,
                "sample",
                parser::get_sample_matches(&command_matches),
            ),
            Some("problems") => {
                Self::from_problems_matches(parser::get_problems_matches(&command_matches))
            }
            Some("problem") => {
                Self::from_problem_matches(parser::get_problem_matches(&command_matches))
            }
            Some("auth") => Self::from_auth_matches(parser::get_auth_matches(&command_matches)),
            Some("init") => Self::from_init_matches(parser::get_init_matches(&command_matches)),
            _ => {
                pretty::print_welcome_message();
                std::process::exit(0);
            }
        }
    }

    fn from_problems_matches(matches: &ArgMatches) -> Self {
        let fields = matches
            .get_many::<String>("field")
            .map(|vals| vals.map(|v| v.to_string()).collect());

        let sort_by = matches.get_one::<String>("sort_by").map(|s| s.to_string());

        Self {
            command_type: CommandType::Problems,
            command_name: "problems".to_string(),
            problem_slug: None,
            code: None,
            dtype: None,
            language: None,
            gpu_type: None,
            fields,
            sort_by,
            description_only: false,
            reference_only: false,
            json_output: parser::get_json_output_flag(matches),
            token: None,
            directory: None,
            all_flag: false,
        }
    }

    fn from_problem_matches(matches: &ArgMatches) -> Self {
        Self {
            command_type: CommandType::Problem,
            command_name: "problem".to_string(),
            problem_slug: Some(parser::get_problem_name(matches).to_string()),
            code: None,
            dtype: None,
            language: None,
            gpu_type: None,
            fields: None,
            sort_by: None,
            description_only: parser::get_description_only_flag(matches),
            reference_only: parser::get_reference_only_flag(matches),
            json_output: parser::get_json_output_flag(matches),
            token: None,
            directory: None,
            all_flag: false,
        }
    }

    fn from_auth_matches(matches: &ArgMatches) -> Self {
        let token = matches.get_one::<String>("token").map(|s| s.to_string());

        Self {
            command_type: CommandType::Auth,
            command_name: "auth".to_string(),
            problem_slug: None,
            code: None,
            dtype: None,
            language: None,
            gpu_type: None,
            fields: None,
            sort_by: None,
            description_only: false,
            reference_only: false,
            json_output: false,
            token,
            directory: None,
            all_flag: false,
        }
    }

    fn from_init_matches(matches: &ArgMatches) -> Self {
        let directory = matches
            .get_one::<String>("directory")
            .map(|s| s.to_string())
            .or(Some(".".to_string())); // default to current dir

        let language = matches.get_one::<String>("language").map(|s| s.to_string());
        let all_flag = matches.get_flag("all");

        let problem = if all_flag {
            None
        } else {
            Some(parser::get_problem_name(matches).to_string())
        };

        Self {
            command_type: CommandType::Init,
            command_name: "init".to_string(),
            problem_slug: problem,
            code: None,
            dtype: None,
            language,
            gpu_type: None,
            fields: None,
            sort_by: None,
            description_only: false,
            reference_only: false,
            json_output: false,
            token: None,
            directory,
            all_flag,
        }
    }

    fn from_subcommand(command_type: CommandType, subcommand: &str, matches: &ArgMatches) -> Self {
        let problem = parser::get_problem_name(matches).to_string();
        let solution_file = parser::get_solution_file(matches);
        let dtype = "float32".to_string();
        let gpu_type = parser::get_gpu_type(matches).to_string();
        let solution_file_extension = solution_file.split('.').last().unwrap();
        let language = match solution_file_extension {
            "py" => "python".to_string(),
            "cu" => "cuda".to_string(),
            "mojo" => "mojo".to_string(),
            _ => "unknown".to_string(),
        };

        let command_name = subcommand.to_string();

        Self {
            command_type,
            command_name,
            problem_slug: Some(problem),
            code: Some(Self::get_file_contents(solution_file)),
            dtype: Some(dtype),
            language: Some(language),
            gpu_type: Some(gpu_type),
            fields: None,
            sort_by: None,
            description_only: false,
            reference_only: false,
            json_output: parser::get_json_output_flag(matches),
            token: None,
            directory: None,
            all_flag: false,
        }
    }

    fn get_file_contents(solution_file: &str) -> String {
        std::fs::read_to_string(solution_file).unwrap()
    }

    // Getters based on command type
    pub fn get_command_name(&self) -> &String {
        &self.command_name
    }

    pub fn get_problem_slug(&self) -> &String {
        self.problem_slug
            .as_ref()
            .expect("Problem slug not available for this command")
    }

    pub fn get_solution_code(&self) -> &String {
        self.code
            .as_ref()
            .expect("Solution code not available for this command")
    }

    pub fn get_gpu_type(&self) -> &String {
        self.gpu_type
            .as_ref()
            .expect("GPU type not available for this command")
    }

    pub fn get_dtype(&self) -> &String {
        self.dtype
            .as_ref()
            .expect("Data type not available for this command")
    }

    pub fn get_language(&self) -> &String {
        self.language
            .as_ref()
            .expect("Language not available for this command")
    }

    pub fn get_directory(&self) -> &String {
        self.directory
            .as_ref()
            .expect("Directory not available for this command")
    }

    pub fn get_fields(&self) -> Option<&Vec<String>> {
        self.fields.as_ref()
    }

    pub fn get_sort_by(&self) -> Option<&String> {
        self.sort_by.as_ref()
    }

    pub fn get_token(&self) -> Option<&String> {
        self.token.as_ref()
    }

    pub fn get_description_only_flag(&self) -> bool {
        self.description_only
    }

    pub fn get_reference_only_flag(&self) -> bool {
        self.reference_only
    }

    pub fn get_json_output_flag(&self) -> bool {
        self.json_output
    }

    pub fn get_all_flag(&self) -> bool {
        self.all_flag
    }

    pub fn is_problem_command(&self) -> bool {
        matches!(
            self.command_type,
            CommandType::Checker
                | CommandType::Benchmark
                | CommandType::Submit
                | CommandType::Sample
        )
    }

    pub fn is_problems_listing(&self) -> bool {
        matches!(self.command_type, CommandType::Problems)
    }

    pub fn is_problem_details(&self) -> bool {
        matches!(self.command_type, CommandType::Problem)
    }

    pub fn is_auth_command(&self) -> bool {
        matches!(self.command_type, CommandType::Auth)
    }
}
