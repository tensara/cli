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

    let mojo_types = |ty: &str| match ty {
        "float" => "Float32".to_string(),
        "double" => "Float64".to_string(),
        "float16" => "Float16".to_string(),
        "float8" => "Float8_e4m3fn".to_string(),
        "float4" => "UInt8".to_string(),
        "int" => "Int32".to_string(),
        "uint8_t" => "UInt8".to_string(),
        "size_t" => "Int64".to_string(),
        "uint32_t" => "UInt32".to_string(),
        "uint64_t" => "UInt64".to_string(),
        _ => ty.to_string(),
    };

    let mojo_dtype_const = |ty: &str| match ty {
        "float" => "DType.float32",
        "double" => "DType.float64",
        "float16" => "DType.float16",
        "float8" => "DType.float8_e4m3fn",
        "float4" => "DType.uint8",
        "int" => "DType.int32",
        "uint8_t" => "DType.uint8",
        "size_t" => "DType.int64",
        "uint32_t" => "DType.uint32",
        "uint64_t" => "DType.uint64",
        _ => "DType.float32",
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

// Note: {} are device pointer parameters
extern \"C\" void solution({}) {{    
}}",
            names.join(", "),
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

# Note: {} are device tensor parameters
def solution({}):
    ",
            names.join(", "),
            param_str
        )
    } else if language == "mojo" {
        let pointer_params: Vec<_> = parameters
            .iter()
            .filter(|p| p.pointer.as_deref() == Some("true"))
            .collect();
        let names: Vec<_> = pointer_params.iter().map(|p| p.name.clone()).collect();

        let mut unique_ptr_types: Vec<String> = Vec::new();
        for parameter in &pointer_params {
            if !unique_ptr_types.contains(&parameter.ty) {
                unique_ptr_types.push(parameter.ty.clone());
            }
        }

        let dtype_var_for_type = |ty: &str| format!("dtype_{}", ty);
        let dtype_block = unique_ptr_types
            .iter()
            .map(|ty| {
                format!(
                    "comptime {} = {}",
                    dtype_var_for_type(ty),
                    mojo_dtype_const(ty)
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let param_str = parameters
            .iter()
            .map(|p| {
                if p.pointer.as_deref() == Some("true") {
                    format!("{}_addr: Int", p.name)
                } else if p.ty == "[VAR]" {
                    format!("{}: {}", p.name, mojo_types(data_type))
                } else {
                    format!("{}: {}", p.name, mojo_types(&p.ty))
                }
            })
            .collect::<Vec<_>>()
            .join(", ");

        let ptr_type_comment = if unique_ptr_types.len() == 1 {
            format!("{} arrays", unique_ptr_types[0])
        } else {
            "mixed-dtype arrays".to_string()
        };

        let pointer_setup = pointer_params
            .iter()
            .map(|p| {
                let var_name = p.name.strip_prefix("d_").unwrap_or(&p.name);
                let dtype_var = dtype_var_for_type(&p.ty);
                format!(
                    "    {} = UnsafePointer[Scalar[{}], MutExternalOrigin](unsafe_from_address={}_addr)",
                    var_name, dtype_var, p.name
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let pointer_setup = if pointer_setup.is_empty() {
            String::new()
        } else {
            format!("{}\n", pointer_setup)
        };

        let dtype_block = if dtype_block.is_empty() {
            String::new()
        } else {
            format!("{}\n", dtype_block)
        };

        format!(
            "from gpu import thread_idx, block_idx, block_dim
from gpu.host import DeviceContext
from memory import UnsafePointer 

{}
# Note: {} are device pointers to {}
@export
def solution({}) raises:
{}    ",
            dtype_block,
            names.join(", "),
            ptr_type_comment,
            param_str,
            pointer_setup
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

#[cfg(test)]
mod tests {
    use super::generate_starter_code;
    use crate::trpc::ProblemParameter;

    #[test]
    fn mojo_starter_uses_address_abi_and_dtype_constants() {
        let parameters = vec![
            ProblemParameter {
                name: "d_input1".to_string(),
                ty: "float".to_string(),
                const_: Some("true".to_string()),
                pointer: Some("true".to_string()),
                constant: None,
            },
            ProblemParameter {
                name: "d_output".to_string(),
                ty: "float".to_string(),
                const_: Some("false".to_string()),
                pointer: Some("true".to_string()),
                constant: None,
            },
            ProblemParameter {
                name: "n".to_string(),
                ty: "size_t".to_string(),
                const_: Some("false".to_string()),
                pointer: Some("false".to_string()),
                constant: None,
            },
        ];

        let starter = generate_starter_code(&parameters, "mojo", "float16");

        assert!(starter.contains("comptime dtype_float = DType.float32"));
        assert!(starter
            .contains("def solution(d_input1_addr: Int, d_output_addr: Int, n: Int64) raises:"));
        assert!(starter.contains(
            "input1 = UnsafePointer[Scalar[dtype_float], MutExternalOrigin](unsafe_from_address=d_input1_addr)"
        ));
        assert!(starter.contains(
            "output = UnsafePointer[Scalar[dtype_float], MutExternalOrigin](unsafe_from_address=d_output_addr)"
        ));
        assert!(!starter.contains("UnsafePointer[Float16]"));
    }
}
