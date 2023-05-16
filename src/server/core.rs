use std::net::Ipv4Addr;

use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::Build;
use rocket::Config;
use rocket::Rocket;

use crate::server::execute;
use crate::server::jwt;

#[get("/")]
fn index() -> &'static str {
    "I am WARK."
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct ServerInfo {
    pub version: String,
    pub commit: String,
    pub data: String,
    pub os: String,
}

#[get("/info")]
fn info() -> Json<ServerInfo> {
    Json(ServerInfo {
        version: env!("VERGEN_GIT_DESCRIBE").to_string(),
        commit: env!("VERGEN_GIT_SHA").to_string(),
        data: env!("VERGEN_GIT_COMMIT_TIMESTAMP").to_string(),
        os: env!("VERGEN_CARGO_TARGET_TRIPLE").to_string(),
    })
}

/// Get the Rocket instance
pub fn rocket() -> Rocket<Build> {
    rocket::build()
        .configure(Config {
            address: Ipv4Addr::new(0, 0, 0, 0).into(),
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "33000".to_string())
                .parse()
                .unwrap(),
            ..Config::default()
        })
        .mount("/", routes![index, info, jwt::validate, execute::execute])
}
