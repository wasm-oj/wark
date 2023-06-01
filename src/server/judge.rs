use crate::judger::{Input, JudgeSpec, Judger, Output};
use crate::run;
use crate::server::jwt;
use base64::engine::general_purpose;
use base64::Engine;
use rocket::serde::{
    json::{Error, Json},
    Deserialize, Serialize,
};
use std::borrow::Cow;
use std::fmt::Debug;
use std::thread;

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct JudgeSubmission {
    wasm: String,
    specs: Vec<JudgeSpec>,
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
}

#[post("/judge", format = "json", data = "<submission>")]
pub async fn judge(
    _token: jwt::Token,
    submission: Result<Json<JudgeSubmission>, Error<'_>>,
) -> Json<JudgeResults> {
    let submission = match submission {
        Ok(submission) => submission.into_inner(),
        Err(e) => {
            let message = format!("Invalid submission. Error parsing JSON: {}", e);
            return Json(JudgeResults {
                results: vec![JudgeResult {
                    success: false,
                    cost: None,
                    memory: None,
                    message: Some(message),
                    exception: None,
                }],
            });
        }
    };

    let wasm = match general_purpose::STANDARD.decode(submission.wasm.as_bytes()) {
        Ok(wasm) => wasm.into_boxed_slice(),
        Err(_) => {
            return Json(JudgeResults {
                results: vec![JudgeResult {
                    success: false,
                    cost: None,
                    memory: None,
                    message: Some("Invalid submission. Error decoding base64.".to_string()),
                    exception: None,
                }],
            });
        }
    };

    let mut results = Vec::new();

    for spec in submission.specs {
        let check = spec.check_spec().await;
        if let Err(e) = check {
            results.push(JudgeResult {
                success: false,
                cost: None,
                memory: None,
                message: None,
                exception: Some(JudgeException::Spec(e)),
            });
            continue;
        }

        let input = spec.make_input().await;
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
        let stdin = input.stdin.clone();

        let (cost_limit, memory_limit) = spec.limits();
        let wasm = wasm.clone();

        let handle = thread::spawn(move || {
            run::run(Cow::Owned(wasm.to_vec()), cost_limit, memory_limit, stdin)
        });

        let result = handle.join().unwrap();

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

    Json(JudgeResults { results })
}
