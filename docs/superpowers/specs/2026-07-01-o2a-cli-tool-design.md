# o2a CLI Tool Design (Minimal Version)

## 1. Core Functionality:
   - The `o2a` tool will parse an OpenAPI JSON specification file provided as a command-line argument.
   - For each distinct path and HTTP method combination defined in the OpenAPI spec, it will generate a standalone executable shell script.
   - The generated scripts will be saved in a structured directory tree, mirroring the API path segments.

## 2. Generated Script Structure and Content:
   - Each generated script will be a `bash` script.
   - **Shebang:** `#!/bin/bash` will be at the top.
   - **Base URL:** The script will `source "$(dirname "$0")/../../config.sh"` to obtain the `BASE_URL`. This `config.sh` file is assumed to exist two directories up from the generated script (e.g., at the project root).
   - **Parameters:**
     - For `path`, `query`, and `header` parameters defined in the OpenAPI spec, corresponding shell variables will be declared at the top of the script (e.g., `STATUS=""`, `ID=""`).
     - These variables will be used directly within the `curl` command (e.g., `.../users/$ID?status=$STATUS`).
     - In this minimal version, these shell variables will be initialized as empty strings (`""`) and will **not** be pre-filled with example values from the OpenAPI spec.
   - **Request Body (for POST, PUT, DELETE, and other methods with `requestBody`):**
     - A shell variable named `REQUEST_BODY` will be declared at the top of the script.
     - If the OpenAPI `requestBody` schema for `application/json` specifies an `example` property, `REQUEST_BODY` will be pre-filled with this example JSON.
     - If no `example` is provided, `REQUEST_BODY` will be initialized to an empty JSON object (`"{}"`).
     - The `curl` command will include `--data "$REQUEST_BODY"` to send the body.
   - **Curl Command:**
     - The core of the script will be a `curl` command including:
       - `--request <HTTP_METHOD>` (e.g., `GET`, `POST`).
       - `--url "<BASE_URL><PATH_WITH_PARAMS><QUERY_STRING>"`
       - `--header "Header-Name: $HEADER_VARIABLE"` for each header parameter.
       - `--data "$REQUEST_BODY"` for methods with a request body.
   - **Minimalism:** This first version will intentionally exclude features like command-line argument parsing within the script, `--help` output, `--dry-run` functionality, or parameter validation. Users are expected to modify the shell variables in the generated script directly or set them via environment variables before execution.

## 3. Output Directory Structure:
   - All generated scripts will be placed within the `generated/` directory at the project root.
   - The directory structure within `generated/` will mirror the API paths. Path parameters (e.g., `{id}`) will be converted into prefixed folder names (e.g., `_id`).
   - Example: An endpoint for `/users/{id}/orders/{orderId}` would generate scripts within `generated/users/_id/orders/_orderId/`.
   - The individual script filename will be the lowercase HTTP method (e.g., `get.sh`, `post.sh`).

## 4. Implementation Approach (within `src/script.rs`):
   - The existing `generate_script` function will be extended to incorporate the logic for handling different HTTP methods.
   - A `match` statement will be used on the `method` argument to conditionally add the `REQUEST_BODY` variable and `--data` flag when generating scripts for methods that support a request body (e.g., POST, PUT, DELETE). This will also include the logic for extracting request body examples from the OpenAPI spec.

## 5. Rust Toolchain:
   - The project will continue to use Rust `edition = "2024"` and will require a nightly Rust toolchain.
