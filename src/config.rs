use std::env;

/// Fetches the maximum computational cost limit from the environment variable "MAX_COST".
/// If the variable is not set or its value cannot be parsed into u64, a default value of 1,000,000,000 is returned.
pub fn max_cost() -> u64 {
    env::var("MAX_COST")
        .unwrap_or("1000000000".to_owned())
        .parse::<u64>()
        .unwrap_or(1000000000)
}

/// Fetches the maximum memory limit from the environment variable "MAX_MEMORY".
/// If the variable is not set or its value cannot be parsed into u32, a default value of 4096 is returned.
pub fn max_memory() -> u32 {
    env::var("MAX_MEMORY")
        .unwrap_or("4096".to_owned())
        .parse::<u32>()
        .unwrap_or(4096)
}

/// Fetches the server port number from the environment variable "PORT".
/// If the variable is not set or its value cannot be parsed into u16, a default value of 33000 is returned.
pub fn server_port() -> u16 {
    env::var("PORT")
        .unwrap_or("33000".to_owned())
        .parse::<u16>()
        .unwrap_or(33000)
}

/// Fetches the application secret from the environment variable "APP_SECRET".
/// If the variable is not set, a default value of "APP_SECRET" is returned.
pub fn app_secret() -> String {
    env::var("APP_SECRET").unwrap_or("APP_SECRET".to_owned())
}
