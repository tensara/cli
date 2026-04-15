# Tensara CLI

Command-line tools for inspecting Tensara problems, running sample checks, checking correctness, benchmarking, and submitting GPU programming solutions.

## Install

Linux/macOS:

```bash
curl -sSL https://get.tensara.org/install.sh | bash
```

Windows:

```powershell
iwr -useb https://get.tensara.org/install.sh | iex
```

## Configuration

The CLI defaults to the production API at `https://tensara.org`.

For local development, set:

```bash
TENSARA_API_BASE_URL=http://localhost:3000
```

You can also override individual routes when debugging:

```bash
CHECKER_ENDPOINT=http://localhost:3000/api/submissions/checker
BENCHMARK_ENDPOINT=http://localhost:3000/api/submissions/benchmark
SUBMIT_ENDPOINT=http://localhost:3000/api/submissions/direct-submit
SAMPLE_ENDPOINT=http://localhost:3000/api/submissions/sample
```

## Authentication

Authenticate with a Tensara API key:

```bash
tensara auth -t <token>
```

API keys are stored in `~/.tensara/auth.json`.

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

Reference only:

```bash
tensara problem vector-addition --reference-only
```

## Run A Sample

Run your solution against the problem sample case:

```bash
tensara sample -g T4 -p vector-addition -s solution.cu
```

The CLI prints the input, expected output, actual output, debug info on failure, and captured stdout/stderr when available.

## Check And Benchmark

Check correctness:

```bash
tensara checker -g T4 -p vector-addition -s solution.cu
```

Benchmark performance:

```bash
tensara benchmark -g T4 -p vector-addition -s solution.cu
```

## Submit

Submit for official evaluation:

```bash
tensara submit -g T4 -p vector-addition -s solution.cu
```

Supported solution file extensions are `.cu`, `.py`, and `.mojo`.

## Init

Generate starter files for a problem:

```bash
tensara init <directory> -p vector-addition -l cuda
```

## Error Handling

The CLI reports HTTP errors before attempting to parse streaming responses. Common cases include:

- `401`: missing or invalid authentication
- `404`: endpoint/backend mismatch
- `429`: rate limit exceeded
- `5xx`: server-side failure

If you see endpoint errors during local development, check `TENSARA_API_BASE_URL` and any per-route overrides.

## Uninstall

Remove the binary:

```bash
sudo rm /usr/local/bin/tensara
```
