pub mod config;
pub mod cost;
mod deterministic_time;
mod memory;
mod random;
pub mod read;
pub mod run;

pub use run::{RunError, RunRequest, RunResult, run};

getrandom::register_custom_getrandom!(random::deterministic_random);

#[cfg(feature = "cli")]
pub mod cli;
#[cfg(feature = "cli")]
pub mod judger;
#[cfg(feature = "cli")]
pub mod server;

#[cfg(feature = "cli")]
#[macro_use]
extern crate rocket;
