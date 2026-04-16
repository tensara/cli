# Tensara CLI

Command-line tools for inspecting Tensara problems, generating starter code, running sample checks, checking correctness, benchmarking, and submitting GPU programming solutions.

The CLI is designed for both humans and coding agents. Human-readable output is the default. Use `--json` when you want machine-readable problem data, sample outputs, checker results, or benchmark metrics.

## Install

Linux/macOS:

```bash
curl -sSL https://get.tensara.org/install.sh | bash
```

Windows:

```powershell
iwr -useb https://get.tensara.org/install.sh | iex
```

Check your installed version:

```bash
tensara --version
```

## Upgrade

Recommended CLI version for the current API:

```text
tensara 0.3.0
```

If you installed from the install script, rerun it:

```bash
curl -sSL https://get.tensara.org/install.sh | bash
```

If you installed with Cargo from crates.io:

```bash
cargo install tensara --force
```

If you installed from a local checkout:

```bash
git pull
cargo install --path . --force
```

## Configuration

The CLI defaults to the production API:

```text
https://tensara.org
```

For local development:

```bash
export TENSARA_API_BASE_URL=http://localhost:3000
```

You can override individual routes while debugging:

```bash
export CHECKER_ENDPOINT=http://localhost:3000/api/submissions/checker
export BENCHMARK_ENDPOINT=http://localhost:3000/api/submissions/benchmark
export SUBMIT_ENDPOINT=http://localhost:3000/api/submissions/direct-submit
export SAMPLE_ENDPOINT=http://localhost:3000/api/submissions/sample
```

## Authentication

Authenticate with a Tensara API key:

```bash
tensara auth -t <token>
```

API keys are stored in:

```text
~/.tensara/auth.json
```

## Problems

List problems:

```bash
tensara problems
```

List problems as JSON:

```bash
tensara problems --json
```

Show one problem, including description and PyTorch reference:

```bash
tensara problem vector-addition
```

Machine-readable problem details:

```bash
tensara problem vector-addition --json
```

The JSON view includes:

- Problem metadata, including `slug`, `title`, `difficulty`, `author`, and `tags`.
- Full problem `description`.
- Full Python problem `definition`.
- Extracted PyTorch `reference_solution`.
- ABI `parameters`.
- Generated starter code under `starters.cuda`, `starters.python`, `starters.mojo`, `starters.cute`, and `starters.cutile`.

Reference only:

```bash
tensara problem vector-addition --reference-only
```

Description only:

```bash
tensara problem vector-addition --description-only
```

## Init

Generate starter files for a problem:

```bash
tensara init ./vector-addition -p vector-addition -l cuda
```

Supported languages:

```bash
tensara init ./vector-addition -p vector-addition -l cuda
tensara init ./vector-addition -p vector-addition -l python
tensara init ./vector-addition -p vector-addition -l mojo
tensara init ./vector-addition -p vector-addition -l cute
tensara init ./vector-addition -p vector-addition -l cutile
```

Generated files include the problem description and starter solution code.

## Run A Sample

Run your solution against the problem sample case:

```bash
tensara sample -g T4 -p vector-addition -s solution.cu
```

Machine-readable sample output:

```bash
tensara sample -g T4 -p vector-addition -s solution.cu --json
```

The sample command prints the input, expected output, actual output, debug info on failure, and captured stdout/stderr when available.

## Check And Benchmark

Check correctness:

```bash
tensara checker -g T4 -p vector-addition -s solution.cu
```

Check correctness as JSON:

```bash
tensara checker -g T4 -p vector-addition -s solution.cu --json
```

Benchmark performance:

```bash
tensara benchmark -g T4 -p vector-addition -s solution.cu
```

Benchmark performance as JSON:

```bash
tensara benchmark -g T4 -p vector-addition -s solution.cu --json
```

## Submit

Submit for official evaluation:

```bash
tensara submit -g T4 -p vector-addition -s solution.cu
```

Supported solution file extensions:

```text
.cu
.py
.mojo
```

Python-based language modes share the `.py` extension. By default, `.py` files are sent as Triton/Python. Use `--language` to submit CuTe DSL or cuTile Python:

```bash
tensara sample -p vector-addition -s sol.cute.py --language cute --json
tensara sample -p vector-addition -s sol.cutile.py --language cutile --json
```

## Using With Agents

Agents should prefer JSON output and generated starters. A reliable agent workflow is:

```bash
tensara problem <slug> --json
tensara init ./workspace -p <slug> -l mojo
tensara sample -p <slug> -s ./workspace/sol.mojo --json
tensara checker -p <slug> -s ./workspace/sol.mojo --json
tensara benchmark -p <slug> -s ./workspace/sol.mojo --json
```

For CUDA:

```bash
tensara init ./workspace -p <slug> -l cuda
tensara sample -p <slug> -s ./workspace/sol.cu --json
```

For Python/Triton:

```bash
tensara init ./workspace -p <slug> -l python
tensara sample -p <slug> -s ./workspace/sol.py --json
```

For CuTe DSL:

```bash
tensara init ./workspace -p <slug> -l cute
tensara sample -p <slug> -s ./workspace/sol.cute.py --language cute --json
```

For cuTile Python:

```bash
tensara init ./workspace -p <slug> -l cutile
tensara sample -p <slug> -s ./workspace/sol.cutile.py --language cutile --json
```

The `problem --json` response is intended to be self-contained for agents. It includes the problem description, PyTorch reference, ABI parameters, and starter code for every supported language.

Agent-specific notes:

- Pointer parameters in CUDA and Mojo starters are device pointers.
- `.py` files default to `python`; pass `--language cute` or `--language cutile` for CuTe DSL and cuTile Python.
- Mojo pointer parameters are passed as raw address `Int` values and reconstructed with `UnsafePointer`.
- Output pointers refer to preallocated output tensors; solutions should write into those outputs.
- If a parameter is a pointer, do not assume it can be read on the host side in a Mojo wrapper. Read device pointer data from inside a launched kernel.
- Use `sample --json` first because it gives the fastest feedback loop.
- Use `checker --json` for full correctness.
- Use `benchmark --json` for runtime and GFLOPS data.

## Error Handling

The CLI reports HTTP errors before attempting to parse streaming responses. Common cases include:

- `401`: missing or invalid authentication.
- `404`: endpoint or backend mismatch.
- `429`: rate limit exceeded.
- `5xx`: server-side failure.

Runtime failures, wrong answers, time limit exceeded, and output limit exceeded statuses are surfaced in the CLI output. Use `--json` for structured status fields that agents can parse.

If you see endpoint errors during local development, check `TENSARA_API_BASE_URL` and any per-route overrides.

## Local Development

Run the CLI from source with arguments after `--`:

```bash
cargo run -- problem vector-addition --json
cargo run -- sample -p vector-addition -s ../actual/tensara/testing/solutions/vector-addition.cu --json
```

Run tests:

```bash
cargo test
```

Format:

```bash
cargo fmt
```

## Uninstall

Remove the binary:

```bash
sudo rm /usr/local/bin/tensara
```
