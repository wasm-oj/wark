# WARK

**W**eb**A**ssembly **R**un**K**it, also known as WARK, is a highly efficient tool designed to execute WebAssembly (w/ WASI) modules within a secure sandboxed environment. It can meticulously calculate and report the precise resource usage of the module, including instruction cost and memory utilization.

You can use WARK as a Command-Line Interface (CLI) tool or as a web service, depending on your needs.

## Table of Contents

- [WARK](#wark)
  - [Table of Contents](#table-of-contents)
  - [Installation](#installation)
    - [Docker](#docker)
    - [Cargo](#cargo)
  - [Usage](#usage)
    - [CLI](#cli)
      - [Options](#options)
      - [IO](#io)
    - [Web Service](#web-service)
  - [Cost Table](#cost-table)

## Installation

### Docker

If you have Docker installed, you can use the following command to run WARK:

```sh
# Run the web service
docker run -it --rm -p 33000:33000 jacoblincool/wark server
```

### Cargo

To install WARK using Cargo, use the following command:

```sh
cargo install wark
```

## Usage

### CLI

To run a WebAssembly module using the CLI, use the following command:

```sh
wark run [OPTIONS] <module>
```

#### Options

You can customize the execution with the following options:

```sh
  -m, --memory <memory>     Define memory limit in MB [default: 512]
  -c, --cost <cost>         Set computational cost limit in instruction count [default: 1000000000]
  -i, --input <input>       Specify input file path for the program [default: stdin]
      --stderr <file>       Redirect program's stderr to a file
  -n, --no-report           Suppress the report of the program's resource usage
```

#### IO

- You can use the `--input` option to specify the input file path for the program. If you want to use stdin as the input, use `-` as the input file path.
- The stdout of the module will be printed to the stdout of the CLI.
- The stderr of the module will **not** be printed to the stderr of the CLI. Instead, use the `--stderr` option to redirect it to a file.
- Unless suppressed with the `--no-report` option, the resource usage of the module will be printed to the stderr of the CLI.

### Web Service

To start the WARK server, use the following command:

```sh
wark server
```

> You can use the `PORT` environment variable to specify the port number. The default port number is `33000`.

To run a WebAssembly module, send a `POST` request with a JSON object in the body containing the following fields:

```sh
curl 'http://127.0.0.1:33000/run' \
  --header 'Content-Type: application/json' \
  --header 'Authorization: Bearer <JWT_TOKEN>' \
  --data '{
    "cost": 10000000,
    "memory": 512,
    "input": "I am stdin input",
    "wasm": "<base64 encoded wasm module>"
  }'
```

The server will respond with a JSON object containing the following fields:

```sh
{
  "success": true,
  "cost": 1234567,
  "memory": 345,
  "stdout": "I am stdout output",
  "stderr": "I am stderr output",
  "message": "I am message"
}
```

## Cost Table

You can find the cost of each instruction in the [src/cost.rs](./src/cost.rs).

> If you see `Penalty Instruction [Instruction Name]`, it means that the specific instruction was not included in the cost table. Therefore, its cost defaults to 1000 points.

---

Feel free to contribute to this project by submitting pull requests or reporting issues. Your help is always welcomed and appreciated!
