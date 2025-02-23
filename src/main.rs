use rocket::tokio::task;
use serde_json::json;
use std::path::PathBuf;
use std::{fs, process};
use std::{io, io::prelude::*};
use tracing::Level;
use tracing_subscriber::FmtSubscriber;
use wark::run::RunRequest;
use wark::*;

#[rocket::main]
async fn main() {
    let matches = cli::cli().get_matches();

    match matches.subcommand() {
        Some(("run", args)) => {
            let mem: u32 = *args
                .get_one("memory")
                .expect("memory limit should be in range 1..");
            let cost: u64 = *args
                .get_one("cost")
                .expect("cost limit should be in range 1..");
            let input: &String = args
                .get_one("input")
                .expect("input file path should be provided");
            let stderr: Option<&PathBuf> = args.get_one("stderr");
            let no_report: &bool = args.get_one("no-report").unwrap_or(&false);
            let module: &PathBuf = args
                .get_one("module")
                .expect("module path should be provided");

            let wasm = read::read_wasm(module.to_path_buf()).expect("Failed to read wasm module");

            let input = match input.as_str() {
                "" => String::new(),
                "-" => {
                    let mut input = String::new();
                    io::stdin()
                        .read_to_string(&mut input)
                        .expect("Failed to read stdin");
                    input
                }
                _ => fs::read_to_string(input).expect("Failed to read input file"),
            };

            let handle = task::spawn_blocking(move || {
                run::run(RunRequest {
                    wasm,
                    budget: cost,
                    mem,
                    input,
                })
            });

            let result = match handle.await.unwrap() {
                Ok(result) => result,
                Err(e) => {
                    eprintln!("{:?}", e);
                    process::exit(1);
                }
            };

            print!(
                "{}",
                String::from_utf8(result.stdout).expect("Failed to convert stdout to string")
            );

            if !no_report {
                let stats = json!({
                    "cost": result.cost,
                    "memory": result.memory,
                });
                eprintln!(
                    "{}",
                    serde_json::to_string_pretty(&stats).expect("Failed to serialize stats")
                );
            }

            if let Some(stderr) = stderr {
                fs::write(stderr, result.stderr).expect("Failed to write stderr to file");
            }
        }
        Some(("server", _)) => {
            match FmtSubscriber::builder()
                .with_max_level(Level::INFO)
                .try_init()
            {
                Ok(_) => (),
                Err(e) => {
                    eprintln!("Failed to initialize tracing: {}", e);
                    process::exit(1);
                }
            }
            let _ = server::core::rocket().launch().await;
        }
        Some(_) | None => {
            let _ = cli::cli().print_help();
        }
    }
}
