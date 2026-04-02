use crate::trpc::get_problem_by_slug;
use crate::trpc::ProblemParameter;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

/*
* Generate starter code from ProblemParameter
*/
pub fn generate_starter_code(
    parameters: &[ProblemParameter],
    language: &str,
    data_type: &str,
) -> String {
    let cpp_types = |dtype: &str| match dtype {
        "float32" => "float",
        "float16" => "double",
        "int32" => "int",
        "int16" => "short",
        _ => "float",
    };

    let python_types = |dtype: &str| match dtype {
        "float32" => "float",
        "float16" => "float16",
        "int32" => "int",
        "int16" => "int16",
        _ => "float",
    };

    let python_misc_types = |ty: &str| match ty {
        "int" => "int",
        "float" => "float",
        "size_t" => "int",
        _ => "int",
    };

    let mojo_types = |dtype: &str| match dtype {
        "float32" => "Float32",
        "float16" => "Float16",
        "int32" => "Int32",
        "int16" => "Int16",
        _ => "Float32",
    };

    let mojo_misc_types = |ty: &str| match ty {
        "int" => "Int32",
        "float" => "Float32",
        "size_t" => "Int32",
        _ => "Int32",
    };

    if language == "cuda" {
        let names: Vec<_> = parameters
            .iter()
            .filter(|p| p.pointer.as_deref() == Some("true"))
            .map(|p| p.name.clone())
            .collect();

        let param_str = parameters
            .iter()
            .map(|p| {
                let mut type_str = if p.ty == "[VAR]" {
                    cpp_types(data_type).to_string()
                } else {
                    p.ty.clone()
                };

                if p.const_.as_deref() == Some("true") {
                    type_str = format!("const {}", type_str);
                }

                if p.pointer.as_deref() == Some("true") {
                    type_str = format!("{}*", type_str);
                }

                format!("{} {}", type_str, p.name)
            })
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            "#include <cuda_runtime.h>

// Note: {} are all device pointers to {} arrays
extern \"C\" void solution({}) {{    
}}",
            names.join(", "),
            data_type,
            param_str
        )
    } else if language == "python" {
        let names: Vec<_> = parameters
            .iter()
            .filter(|p| p.pointer.as_deref() == Some("true"))
            .map(|p| p.name.clone())
            .collect();

        let param_str = parameters
            .iter()
            .map(|p| {
                if p.pointer.as_deref() == Some("true") {
                    p.name.clone()
                } else if p.ty == "[VAR]" {
                    format!("{}: {}", p.name, python_types(data_type))
                } else {
                    format!("{}: {}", p.name, python_misc_types(&p.ty))
                }
            })
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            "import triton\nimport triton.language as tl

# Note: {} are all {} device tensors
def solution({}):
    ",
            names.join(", "),
            data_type,
            param_str
        )
    } else if language == "mojo" {
        let names: Vec<_> = parameters
            .iter()
            .filter(|p| p.pointer.as_deref() == Some("true"))
            .map(|p| p.name.clone())
            .collect();

        let param_str = parameters
            .iter()
            .map(|p| {
                let type_str = if p.pointer.as_deref() == Some("true") {
                    format!("UnsafePointer[{}]", mojo_types(data_type))
                } else if p.ty == "[VAR]" {
                    mojo_types(data_type).to_string()
                } else {
                    mojo_misc_types(&p.ty).to_string()
                };

                format!("{}: {}", p.name, type_str)
            })
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            "from gpu.host import DeviceContext
from gpu.id import block_dim, block_idx, thread_idx
from memory import UnsafePointer

# Note: {} are all device pointers to {} arrays
@export
fn solution({}) raises:
    ",
            names.join(", "),
            data_type,
            param_str
        )
    } else {
        "".to_string()
    }
}

pub fn validate_code(code: &str, language: &str) -> Result<(), String> {
    if language == "python" {
        if code.contains("torch.") || code.contains("import torch") {
            return Err("You cannot use PyTorch in the code!".into());
        }

        let exec_regex = regex::Regex::new(r"exec\s*\(\s*[^)]*\)").unwrap();
        if exec_regex.is_match(code) {
            return Err("You cannot use exec() in the code!".into());
        }
    }

    Ok(())
}

/*
* Generates a comment block for the given problem description
*/

pub fn generate_comment_block(description: &str, language: &str) -> String {
    let comment_prefix = match language {
        "cuda" => "// ",
        "python" => "# ",
        "mojo" => "# ",
        _ => "// ",
    };

    let lines = description.lines();

    let mut result = String::new();
    result.push_str(&format!(
        "{}{}\n",
        comment_prefix.trim(),
        "Problem Description"
    ));

    for line in lines {
        // can add logic to format markdown headers
        if line.trim().is_empty() {
            result.push_str(&format!("{}\n", comment_prefix));
        } else {
            result.push_str(&format!("{}{}\n", comment_prefix, line));
        }
    }

    result
}

pub fn write_problem_markdown_file(path: &Path, description: &str) -> std::io::Result<()> {
    let md_path = path.join("PROBLEM.md");
    fs::write(md_path, description)
}

/*
* TODO Create sol.cu with input string in the given directory
*/

pub fn init(
    path: &Path,
    language: &str,
    problem_slug: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if !path.exists() {
        fs::create_dir_all(path)?;
    }

    let result = get_problem_by_slug(problem_slug)?;
    let description = result.description.unwrap_or_default();
    let parameters = result.parameters.unwrap_or_default();

    let data_type = "float16";

    write_problem_markdown_file(path, &description)?;

    let comment_block = generate_comment_block(&description, language);
    let starter_code = generate_starter_code(&parameters, language, data_type);

    let full_code = format!("{comment_block}\n\n{starter_code}");

    let file_name = match language {
        "cuda" => "sol.cu",
        "python" => "sol.py",
        "mojo" => "sol.mojo",
        _ => "sol.txt",
    };

    let file_path = path.join(file_name);
    let mut file = File::create(file_path)?;
    file.write_all(full_code.as_bytes())?;

    println!(
        "✅ Generated starter code and description for problem `{}`",
        problem_slug
    );
    println!("📁 Directory: {}", path.display());

    Ok(())
}
