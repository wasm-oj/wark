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
      - [Run](#run)
      - [Judge](#judge)
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

#### Run

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

```json
{
    "success": true,
    "cost": 1234567,
    "memory": 345,
    "stdout": "I am stdout output",
    "stderr": "I am stderr output",
    "message": "I am message"
}
```

#### Judge

To run the program in judge mode, send a `POST` request with a JSON object in the body containing the following fields:

```sh
curl --location 'http://127.0.0.1:33000/judge' \
--header 'Content-Type: application/json' \
--header 'Authorization: Bearer <JWT_TOKEN>' \
--data '{
    "wasm": "<base64 encoded wasm module>",
    "specs": [
        {
            "judger": "IOFast",
            "input": "Jacob",
            "output_hash": "128783a055c41c0a79ad7376e8e22587cdca53ed1f9c47c46d02a7768992b325",
            "cost": 1000000000,
            "memory": 1024
        },
        {
            "judger": "IOFast",
            "input": "WOJ",
            "output_hash": "75787b1df461d0c48f0229a7769cbcc37c7d96d6613f825b77e76afdd1eb790a",
            "cost": 1000000000,
            "memory": 1024
        },
        {
            "judger": "IOFast",
            "input": "WASM OJ Wonderland",
            "output_hash": "8f02d3283b88d16766cb287090bf59135c873e9175759b73f96ffe674440ff21",
            "cost": 1000000000,
            "memory": 1024
        },
        {
            "judger": "IOFast",
            "input_url": "https://link-to-input.file/input.txt",
            "output_hash": "87c215c4afeaf7ff7684ef90fd44649b2051bc4c68cf58bdad402fa304487b8w",
            "cost": 1000000000,
            "memory": 1024
        }
    ]
}'
```

The server will respond with a JSON object containing the following fields:

```json
{
    "results": [
        {
            "success": true,
            "cost": 3776,
            "memory": 1,
            "message": null,
            "exception": null
        },
        {
            "success": true,
            "cost": 3692,
            "memory": 1,
            "message": null,
            "exception": null
        },
        {
            "success": true,
            "cost": 4421,
            "memory": 1,
            "message": null,
            "exception": null
        },
        {
            "success": false,
            "cost": 5848,
            "memory": 1,
            "message": null,
            "exception": {
                "type": "Output",
                "reason": "Output hash mismatch. Expected 87c215c4afeaf7ff7684ef90fd44649b2051bc4c68cf58bdad402fa304487b8w, got 87c215c4afeaf7ff7684ef90fd44649b2051bc4c68cf58bdad402fa304487b8c"
            }
        }
    ]
}
```

Currently, the server only supports the `IOFast` judger, which is a simple judger that compares the trimmed output of the program with the `output_hash` field. If the output of the program matches the `output_hash` field, indicating that the program has passed the test case. Otherwise, an `Output` exception will be returned.

> Remote inputs will be cached in the `http-cache` directory, the TTL of each cache is respecting the `Cache-Control` header of the response.

## Cost Table

You can find the cost of each instruction in the [src/cost.rs](./src/cost.rs).

> If you see `Penalty Instruction [Instruction Name]`, it means that the specific instruction was not included in the cost table. Therefore, its cost defaults to 1000 points.

---

Feel free to contribute to this project by submitting pull requests or reporting issues. Your help is always welcomed and appreciated!
