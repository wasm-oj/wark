use super::jwt;
use crate::config::*;
use crate::run;
use base64::{engine::general_purpose, Engine as _};
use rocket::serde::{
    json::{Error, Json},
    Deserialize, Serialize,
};
use rocket::tokio::task;

// Define a struct to represent incoming code submissions
#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Submission {
    wasm: String,
    input: String,
    cost: u64,
    memory: u32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct ExecutionResult {
    success: bool,
    cost: Option<u64>,
    memory: Option<u32>,
    stdout: Option<String>,
    stderr: Option<String>,
    message: Option<String>,
}

// Define a Rocket route to handle incoming code submissions
#[post("/run", format = "json", data = "<submission>")]
pub async fn execute(
    _token: jwt::Token,
    submission: Result<Json<Submission>, Error<'_>>,
) -> Json<ExecutionResult> {
    let submission = match submission {
        Ok(submission) => submission.into_inner(),
        Err(e) => {
            let message = format!("Invalid submission. Error parsing JSON: {}", e);
            return Json(ExecutionResult {
                success: false,
                cost: None,
                memory: None,
                stdout: None,
                stderr: None,
                message: Some(message),
            });
        }
    };

    if submission.cost > max_cost() {
        return Json(ExecutionResult {
            success: false,
            cost: None,
            memory: None,
            stdout: None,
            stderr: None,
            message: Some("Invalid cost limit".to_string()),
        });
    }

    if submission.memory > max_memory() {
        return Json(ExecutionResult {
            success: false,
            cost: None,
            memory: None,
            stdout: None,
            stderr: None,
            message: Some("Invalid memory limit".to_string()),
        });
    }

    let wasm = match general_purpose::STANDARD.decode(submission.wasm.as_bytes()) {
        Ok(wasm) => wasm.into_boxed_slice(),
        Err(_) => {
            return Json(ExecutionResult {
                success: false,
                cost: None,
                memory: None,
                stdout: None,
                stderr: None,
                message: Some("Invalid wasm".to_string()),
            })
        }
    };

    let handle = task::spawn_blocking(move || {
        run::run(wasm, submission.cost, submission.memory, submission.input)
    });

    let result = handle.await.unwrap();

    match result {
        Ok(result) => {
            Json(ExecutionResult {
                success: true,
                cost: Some(result.cost),
                memory: Some(result.memory),
                stdout: Some(String::from_utf8(result.stdout).unwrap_or(
                    "Failed to decode stdout, it may contain invalid UTF-8".to_string(),
                )),
                stderr: Some(String::from_utf8(result.stderr).unwrap_or(
                    "Failed to decode stderr, it may contain invalid UTF-8".to_string(),
                )),
                message: None,
            })
        }
        Err(err) => Json(ExecutionResult {
            success: false,
            cost: None,
            memory: None,
            stdout: None,
            stderr: None,
            message: Some(format!("{:?}", err)),
        }),
    }
}
