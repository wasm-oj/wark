use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub mod io_fast;

#[derive(Debug, Serialize, Deserialize)]
pub struct Input {
    pub stdin: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Output {
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "judger")]
pub enum JudgeSpec {
    IOFast(io_fast::FastIOJudgeSpec),
}

#[async_trait]
pub trait Judger: Debug {
    async fn check_spec(&self) -> Result<(), String>;
    async fn make_input(&self) -> Result<Input, String>;
    async fn judge_output(&self, input: &Input, output: &Output) -> Result<(), String>;
    fn limits(&self) -> (u64, u32);
}

#[async_trait]
impl Judger for JudgeSpec {
    async fn check_spec(&self) -> Result<(), String> {
        match self {
            JudgeSpec::IOFast(io_fast_spec) => io_fast_spec.check_spec().await,
        }
    }

    async fn make_input(&self) -> Result<Input, String> {
        match self {
            JudgeSpec::IOFast(io_fast_spec) => io_fast_spec.make_input().await,
        }
    }

    async fn judge_output(&self, input: &Input, output: &Output) -> Result<(), String> {
        match self {
            JudgeSpec::IOFast(io_fast_spec) => io_fast_spec.judge_output(input, output).await,
        }
    }

    fn limits(&self) -> (u64, u32) {
        match self {
            JudgeSpec::IOFast(io_fast_spec) => io_fast_spec.limits(),
        }
    }
}
