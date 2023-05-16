use std::env;

pub fn max_cost() -> u64 {
    env::var("MAX_COST")
        .unwrap_or("1000000000".to_owned())
        .parse::<u64>()
        .unwrap_or(1000000000)
}

pub fn max_memory() -> u32 {
    env::var("MAX_MEMORY")
        .unwrap_or("4096".to_owned())
        .parse::<u32>()
        .unwrap_or(4096)
}

pub fn server_port() -> u16 {
    env::var("PORT")
        .unwrap_or("33000".to_owned())
        .parse::<u16>()
        .unwrap_or(33000)
}

pub fn app_secret() -> String {
    env::var("APP_SECRET").unwrap_or("APP_SECRET".to_owned())
}
