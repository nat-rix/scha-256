use crate::Error;
use std::lazy::SyncLazy;

use rocket::config::{Environment, LoggingLevel};
use rocket::http::RawStr;
use rocket::request::Form;
use rocket::response::{content::Html, Redirect};
use rocket::Request;

pub struct Templates {
    base: liquid::Template,
    index: liquid::Template,
    s404: liquid::Template,
    s500: liquid::Template,
}

impl Templates {
    pub fn new() -> Result<Self, Error> {
        let err = |err| Error::TemplateParsingError(Box::new(err));
        let parser = liquid::ParserBuilder::with_stdlib().build().unwrap();
        Ok(Self {
            base: parser
                .parse(include_str!("templates/base.html"))
                .map_err(err)?,
            index: parser
                .parse(include_str!("templates/index.html"))
                .map_err(err)?,
            s404: parser
                .parse(include_str!("templates/404.html"))
                .map_err(err)?,
            s500: parser
                .parse(include_str!("templates/500.html"))
                .map_err(err)?,
        })
    }

    fn parsing_err<E: std::error::Error + 'static>(err: E) -> Error {
        Error::TemplateRenderingError(Box::new(err))
    }

    fn get_index_container(&self) -> Result<String, Error> {
        self.index
            .render(&liquid::object! {{
            }})
            .map_err(Self::parsing_err)
    }

    fn get_base(&self, title: &str, content: Result<String, Error>) -> Result<String, Error> {
        self.base
            .render(&liquid::object! {{
                "title": title,
                "container": &content?,
            }})
            .map_err(Self::parsing_err)
    }

    pub fn get_index(&self) -> Result<String, Error> {
        self.get_base("Start Page", self.get_index_container())
    }

    pub fn get_404(&self, url: &str) -> Result<String, Error> {
        self.get_base(
            "Error",
            self.s404
                .render(&liquid::object! {{
                    "url": url
                }})
                .map_err(Self::parsing_err),
        )
    }

    pub fn get_500(&self, err: &str) -> Result<String, Error> {
        self.get_base(
            "Error",
            self.s500
                .render(&liquid::object! {{
                    "error": err
                }})
                .map_err(Self::parsing_err),
        )
    }
}

#[catch(500)]
fn internal_error(req: &Request) -> Html<String> {
    Html(TEMPLATES.get_500("").unwrap())
}

#[catch(404)]
fn not_found(req: &Request) -> Html<String> {
    Html(TEMPLATES.get_404(req.uri().path()).unwrap())
}

#[get("/")]
fn index() -> Html<String> {
    Html(TEMPLATES.get_index().unwrap())
}

#[derive(FromForm, UriDisplayQuery, Debug, Clone)]
pub struct GameCreationForm {
    human1: bool,
    human2: bool,
}

#[post("/game", data = "<game>")]
fn new_game(game: Form<GameCreationForm>) -> Redirect {
    let id = 0;
    Redirect::to(format!("/game/{}", id))
}

static TEMPLATES: SyncLazy<Templates> = SyncLazy::new(|| Templates::new().unwrap());

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
            .mount(&config.root, routes![index, new_game])
            .launch(),
    )
}
