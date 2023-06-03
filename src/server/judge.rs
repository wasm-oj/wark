use crate::judger::{Input, JudgeSpec, Judger, Output};
use crate::run;
use crate::server::jwt;
use base64::engine::general_purpose;
use base64::Engine;
use reqwest::Client;
use rocket::serde::{
    json::{Error, Json},
    Deserialize, Serialize,
};
use rocket::tokio::task;
use std::fmt::Debug;

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct JudgeSubmission {
    /// The base64-encoded WebAssembly binary
    wasm: String,
    /// Judge specifications
    specs: Vec<JudgeSpec>,
    /// Callback URL to send the results to (optional)
    callback: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
#[serde(tag = "type", content = "reason")]
pub enum JudgeException {
    Spec(String),
    Input(String),
    Execution(String),
    Output(String),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct JudgeResult {
    success: bool,
    cost: Option<u64>,
    memory: Option<u32>,
    message: Option<String>,
    exception: Option<JudgeException>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct JudgeResults {
    results: Vec<JudgeResult>,
    error: Option<String>,
}

#[post("/judge", format = "json", data = "<submission>")]
pub async fn judge(
    _token: jwt::Token,
    submission: Result<Json<JudgeSubmission>, Error<'_>>,
) -> Json<JudgeResults> {
    info!("Received judge request");
    let submission = match submission {
        Ok(submission) => submission.into_inner(),
        Err(e) => {
            info!("Bad judge request: {}", e);
            return Json(JudgeResults {
                results: vec![],
                error: Some(format!("Invalid submission. Error parsing JSON: {}", e)),
            });
        }
    };

    let wasm = match general_purpose::STANDARD.decode(submission.wasm.as_bytes()) {
        Ok(wasm) => wasm.into_boxed_slice(),
        Err(_) => {
            info!("Bad judge request: invalid base64 encoding");
            return Json(JudgeResults {
                results: vec![],
                error: Some("Invalid submission. Error decoding base64.".to_string()),
            });
        }
    };

    if let Some(callback) = submission.callback {
        task::spawn(async move {
            let result = run_specs(wasm, submission.specs).await;
            let client = Client::new();
            match client.post(&callback).json(&result).send().await {
                Ok(_) => {
                    println!("Callback sent successfully. ({})", &callback);
                }
                Err(e) => {
                    println!("Error sending callback. {} ({})", e, &callback);
                }
            }
        });

        Json(JudgeResults {
            results: vec![],
            error: None,
        })
    } else {
        let result = run_specs(wasm, submission.specs).await;
        Json(result)
    }
}

pub async fn run_specs(wasm: Box<[u8]>, specs: Vec<JudgeSpec>) -> JudgeResults {
    let mut tasks = Vec::new();

    for spec in specs {
        let wasm = wasm.clone();
        let task = task::spawn(async move {
            let check = spec.check_spec().await;
            if let Err(e) = check {
                return (Err(e), Err("".to_string()), None);
            }

            let input = spec.make_input().await;
            if let Err(e) = input {
                return (Ok(spec), Err(e), None);
            }
            let input = input.unwrap();
            let stdin = input.stdin.clone();

            let (cost_limit, memory_limit) = spec.limits();

            let task = task::spawn_blocking(move || {
                info!("Running judge for spec: {:?}", spec);
                let result = run::run(wasm, cost_limit, memory_limit, stdin);
                info!("Judge finished for spec: {:?}", spec);
                (Ok(spec), Ok(input), Some(result))
            });

            task.await.unwrap()
        });
        tasks.push(task);
    }

    let mut results = Vec::new();

    for task in tasks {
        let (spec, input, result) = task.await.unwrap();
        if let Err(e) = spec {
            results.push(JudgeResult {
                success: false,
                cost: None,
                memory: None,
                message: None,
                exception: Some(JudgeException::Spec(e)),
            });
            continue;
        }
        let spec = spec.unwrap();

        if let Err(e) = input {
            results.push(JudgeResult {
                success: false,
                cost: None,
                memory: None,
                message: None,
                exception: Some(JudgeException::Input(e)),
            });
            continue;
        }
        let input = input.unwrap();

        let result = result.unwrap();

        match result {
            Ok(result) => {
                let success = spec
                    .judge_output(
                        &Input { stdin: input.stdin },
                        &Output {
                            stdout: String::from_utf8(result.stdout).unwrap(),
                            stderr: String::from_utf8(result.stderr).unwrap(),
                        },
                    )
                    .await;
                if let Err(e) = success {
                    results.push(JudgeResult {
                        success: false,
                        cost: Some(result.cost),
                        memory: Some(result.memory),
                        message: None,
                        exception: Some(JudgeException::Output(e)),
                    });
                    continue;
                }

                results.push(JudgeResult {
                    success: true,
                    cost: Some(result.cost),
                    memory: Some(result.memory),
                    message: None,
                    exception: None,
                });
            }
            Err(e) => {
                let exception = match e {
                    run::RunError::SpendingLimitExceeded(_) => "SLE",
                    run::RunError::MemoryLimitExceeded(_) => "MLE",
                    run::RunError::RuntimeError(_) => "RE",
                    run::RunError::CompileError(_) => "CE",
                    run::RunError::IOError(_) => "IOE",
                };
                results.push(JudgeResult {
                    success: false,
                    cost: None,
                    memory: None,
                    message: None,
                    exception: Some(JudgeException::Execution(exception.to_string())),
                });
            }
        }
    }

    JudgeResults {
        results,
        error: None,
    }
}
