use super::{Input, Judger, Output};
use async_trait::async_trait;
use http_cache_reqwest::{CACacheManager, Cache, CacheMode, HttpCache};
use reqwest::Client;
use reqwest_middleware::ClientBuilder;
use serde::{Deserialize, Serialize};
use sha256::digest;

#[derive(Debug, Serialize, Deserialize)]
pub struct FastIOJudgeSpec {
    /// The input string
    pub input: Option<String>,
    /// The URL to fetch the input from
    pub input_url: Option<String>,
    /// The token used to authenticate the input URL
    pub input_auth: Option<String>,
    /// The expected output hash
    pub output_hash: String,
    /// The maximum cost of the program
    pub cost: u64,
    /// The maximum memory of the program
    pub memory: u32,
}

#[async_trait]
impl Judger for FastIOJudgeSpec {
    async fn check_spec(&self) -> Result<(), String> {
        if self.cost > 1000000000 {
            return Err(format!(
                "Invalid cost limit, got {}, max is 1,000,000,000",
                self.cost
            ));
        }

        if self.memory > 2048 {
            return Err(format!(
                "Invalid memory limit, got {}, max is 2,048",
                self.memory
            ));
        }

        if self.input.is_none() && self.input_url.is_none() {
            return Err("Must provide either input or input_url".to_string());
        }

        Ok(())
    }

    async fn make_input(&self) -> Result<Input, String> {
        if let Some(input) = &self.input {
            return Ok(Input {
                stdin: input.clone(),
            });
        }

        if let Some(input_url) = &self.input_url {
            let client = ClientBuilder::new(Client::new())
                .with(Cache(HttpCache {
                    mode: CacheMode::Default,
                    manager: CACacheManager::default(),
                    options: None,
                }))
                .build();

            let mut req = client.get(input_url);
            if let Some(auth) = &self.input_auth {
                req = req.header("Authorization", format!("Bearer {}", auth));
            }

            let res = req
                .send()
                .await
                .map_err(|e| format!("Error fetching input: {}", e))?;

            let input = res
                .text()
                .await
                .map_err(|e| format!("Error reading input: {}", e))?;

            return Ok(Input { stdin: input });
        }

        unreachable!()
    }

    async fn judge_output(&self, _input: &Input, output: &Output) -> Result<(), String> {
        let output_hash = digest(
            output
                .stdout
                .lines()
                .map(|l| l.trim_end())
                .collect::<Vec<_>>()
                .join("\n")
                .trim()
                .as_bytes(),
        );

        if output_hash != self.output_hash {
            return Err(format!(
                "Output hash mismatch. Expected {}, got {}",
                self.output_hash, output_hash
            ));
        }

        Ok(())
    }

    fn limits(&self) -> (u64, u32) {
        (self.cost, self.memory)
    }
}
