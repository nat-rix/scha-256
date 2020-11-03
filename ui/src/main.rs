#![feature(proc_macro_hygiene, decl_macro, once_cell)]

#[macro_use]
extern crate rocket;
extern crate scha256_engine as engine;

pub mod error;
pub mod server;

pub use error::Error;

use clap::Arg;
use rocket::config::LoggingLevel;
use server::ServerConfig;

fn args_as_config() -> Result<ServerConfig, Error> {
    let matches = clap::app_from_crate!()
        .arg(
            Arg::new("address")
                .long("addr")
                .alias("address")
                .short('a')
                .value_name("ADDRESS")
                .default_value("localhost"),
        )
        .arg(
            Arg::new("port")
                .long("port")
                .short('p')
                .value_name("PORT")
                .default_value("8080"),
        )
        .arg(
            Arg::new("white")
                .long("white")
                .short('w')
                .possible_values(&["ai", "ui"])
                .value_name("OPPONENT")
                .default_value("ui"),
        )
        .arg(
            Arg::new("black")
                .long("black")
                .short('b')
                .possible_values(&["ai", "ui"])
                .value_name("OPPONENT")
                .default_value("ai"),
        )
        .arg(
            Arg::new("log-level")
                .long("log-level")
                .alias("log")
                .short('l')
                .case_insensitive(true)
                .possible_values(&["off", "critical", "info", "debug"])
                .value_name("LEVEL")
                .default_value("info"),
        )
        .arg(
            Arg::new("root")
                .long("root")
                .short('r')
                .value_name("LOCATION")
                .default_value("/"),
        )
        .get_matches();
    Ok(server::ServerConfig {
        addr: matches.value_of("address").unwrap().to_string(),
        port: matches
            .value_of_t("port")
            .map_err(|e| Error::ArgumentParsingError(Box::new(e)))?,
        log_level: match matches.value_of("log-level").unwrap() {
            "off" => LoggingLevel::Off,
            "critical" => LoggingLevel::Critical,
            "info" => LoggingLevel::Normal,
            "debug" => LoggingLevel::Debug,
            _ => unreachable!("invalid value for log-level"),
        },
        root: matches.value_of("root").unwrap().to_string(),
    })
}

fn main() {
    match args_as_config() {
        Ok(cfg) => {
            let handle = std::thread::spawn(|| {
                let e = server::launch(cfg);
                println!("{}", e)
            });
            handle.join().unwrap();
        }
        Err(e) => println!("{}", e),
    }
}
