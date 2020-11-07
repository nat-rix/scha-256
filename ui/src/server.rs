use crate::{
    templates::{Templates, TEMPLATES},
    Error,
};
use engine::chessmatch::MatchRegistry;
use std::lazy::SyncLazy;

use rocket::config::{Environment, LoggingLevel};
use rocket::http::RawStr;
use rocket::request::Form;
use rocket::response::{content::Html, Redirect};
use rocket::Request;

static MATCH_REGISTRY: SyncLazy<MatchRegistry<Match>> = SyncLazy::new(|| MatchRegistry::new());

#[derive(Debug, Clone)]
pub struct Match {
    host_color: engine::board::Color,
    white_human: bool,
    black_human: bool,
}

#[derive(FromForm, UriDisplayQuery, Debug, Clone)]
pub struct MatchCreationForm {
    human1: bool,
    human2: bool,
    hostcolor: bool,
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

#[post("/match", data = "<desc>")]
fn new_match(desc: Form<MatchCreationForm>) -> Redirect {
    let id = MATCH_REGISTRY.create_match(Match {
        host_color: match desc.hostcolor {
            true => engine::board::Color::White,
            false => engine::board::Color::Black,
        },
        white_human: desc.human1,
        black_human: desc.human2,
    });
    Redirect::to(format!("/match/{}/host", id))
}

#[derive(Debug, Clone, Copy)]
pub enum UserType {
    Host,
    Player,
    Spectator,
}

pub struct RequestWrap<'a, 'r>(&'a Request<'r>);
impl<'a, 'r> rocket::request::FromRequest<'a, 'r> for RequestWrap<'a, 'r> {
    type Error = ();
    fn from_request(req: &'a Request<'r>) -> rocket::request::Outcome<Self, ()> {
        rocket::request::Outcome::Success(RequestWrap(req))
    }
}

#[get("/match/<id>/<user>")]
fn view_match(req: RequestWrap, id: u32, user: String) -> Result<Html<String>, Html<String>> {
    let user = match user.as_str() {
        "host" => UserType::Host,
        "player" => UserType::Player,
        "spectator" => UserType::Spectator,
        _ => return Err(Html(TEMPLATES.get_404(req.0.uri().path()).unwrap())),
    };
    let reg = SyncLazy::force(&MATCH_REGISTRY);
    let board = reg
        .get_board(id)
        .ok_or_else(|| Html(TEMPLATES.get_404(req.0.uri().path()).unwrap()))?;
    Ok(Html(TEMPLATES.get_chessboard(&board).unwrap()))
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
            .mount(&config.root, routes![index, new_match, view_match])
            .launch(),
    )
}
