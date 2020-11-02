use crate::Error;

use rocket::config::{Config, Environment, LoggingLevel};
use rocket::Request;

#[catch(500)]
fn internal_error() -> &'static str {
    "Whoops! Looks like we messed up."
}

#[catch(404)]
fn not_found(req: &Request) -> String {
    format!("I couldn't find '{}'. Try something else?", req.uri())
}

#[get("/")]
fn index() -> &'static str {
    "hello world"
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub addr: String,
    pub port: u16,
    pub log_level: LoggingLevel,
    pub root: String,
}

pub fn launch(config: ServerConfig) -> Error {
    let app = rocket::custom(
        match rocket::config::Config::build(Environment::Production)
            .address(config.addr)
            .port(config.port)
            .log_level(config.log_level)
            .finalize()
        {
            Err(e) => return Error::InvalidServerConfiguration(Box::new(e)),
            Ok(v) => v,
        },
    );
    Error::LaunchError(
        app.register(catchers![internal_error, not_found])
            .mount(&config.root, routes![index])
            .launch(),
    )
}
