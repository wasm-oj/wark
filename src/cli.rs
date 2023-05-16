use std::path::PathBuf;

use clap::{arg, value_parser, Command};

pub fn cli() -> Command {
    Command::new("wark")
        .version(format!(
            "{} {}",
            env!("VERGEN_GIT_SHA"),
            env!("VERGEN_CARGO_TARGET_TRIPLE")
        ))
        .about("WebAssembly RunKit")
        .author("Jacob Lin <jacob@csie.cool>")
        .subcommand(Command::new("server").about("Run the WARK server."))
        .subcommand(
            Command::new("run")
                .about("Run a WebAssembly module with limitations")
                .args(&[
                    arg!(-m --memory <memory> "memory limit in MB")
                        .default_value("512")
                        .value_parser(value_parser!(u32).range(1..)),
                    arg!(-c --cost <cost> "computational cost limit in instruction count")
                        .default_value("1000000000")
                        .value_parser(value_parser!(u64).range(1..)),
                    arg!(-i --input <input> "input file path to the program")
                        .default_value("")
                        .value_parser(value_parser!(String)),
                    arg!(--stderr <file> "redirect program's stderr to a file")
                        .value_parser(value_parser!(PathBuf)),
                    arg!(-n --"no-report" "do not report the program's resource usage")
                        .value_parser(value_parser!(bool)),
                    arg!(<module> "a path to WebAssembly module (.wasm)")
                        .value_parser(value_parser!(PathBuf)),
                ]),
        )
}
