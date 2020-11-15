use crate::{
    templates::{Templates, TEMPLATES},
    Error,
};
use engine::board::{Color, Coord, Piece};
use engine::chessmatch::{MatchInfos, MatchRegistry};
use engine::moves::{Move, MoveType};
use std::lazy::SyncLazy;
use std::str::FromStr;

use rocket::config::{Environment, LoggingLevel};
use rocket::http::RawStr;
use rocket::request::Form;
use rocket::response::{content::Html, status::NotFound, Redirect};
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
fn not_found(req: &Request) -> NotFound<Html<String>> {
    NotFound(Html(TEMPLATES.get_404(req.uri().path()).unwrap()))
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
        white_human: !desc.human1,
        black_human: !desc.human2,
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

fn get_user(user: &str, req: &Request) -> Result<UserType, NotFound<Html<String>>> {
    match user {
        "host" => Ok(UserType::Host),
        "player" => Ok(UserType::Player),
        "spectator" => Ok(UserType::Spectator),
        _ => Err(not_found(req)),
    }
}

fn parse_coord(coord: &str) -> Option<Coord> {
    Coord::from_str(coord).ok()
}

fn color_to_move(user: UserType, info: &MatchInfos<Match>) -> bool {
    matches!((user, info.color, info.extra.host_color, info.extra.white_human, info.extra.black_human),
        (UserType::Host, Color::White, Color::White, true, _)
        | (UserType::Host, Color::Black, Color::Black, _, true)
        | (UserType::Player, Color::White, Color::Black, true, _)
        | (UserType::Player, Color::Black, Color::White, _, true))
}

#[get("/favicon.ico")]
fn favicon() {}

#[get("/match/<id>/<userstr>")]
fn view_match(
    req: RequestWrap,
    id: u32,
    userstr: String,
) -> Result<Html<String>, NotFound<Html<String>>> {
    let user = get_user(&userstr, req.0)?;
    let reg = SyncLazy::force(&MATCH_REGISTRY);
    let board = reg.get_board(id).ok_or_else(|| not_found(req.0))?;
    let info = reg.get_info(id).ok_or_else(|| not_found(&req.0))?;
    Ok(Html(
        TEMPLATES
            .get_chessboard(
                format!("/match/{}/{}", id, userstr),
                "",
                &board,
                info.result,
                None,
            )
            .unwrap(),
    ))
}

#[get("/match/<id>/<userstr>/<coordstr>")]
fn view_match_select(
    req: RequestWrap,
    id: u32,
    userstr: String,
    coordstr: String,
) -> Result<Html<String>, NotFound<Html<String>>> {
    let user = get_user(&userstr, req.0)?;
    let coord = parse_coord(&coordstr).ok_or_else(|| not_found(&req.0))?;
    let reg = SyncLazy::force(&MATCH_REGISTRY);
    let board = reg.get_board(id).ok_or_else(|| not_found(&req.0))?;
    let info = reg.get_info(id).ok_or_else(|| not_found(&req.0))?;
    let moves = if color_to_move(user, &info) {
        let flatten = |m: Coord| {
            let (x, y) = m.as_xy();
            y * 8 + x
        };
        Some((
            flatten(coord),
            board
                .enumerate_moves(info.color, coord)
                .slice()
                .iter()
                .filter(|m| board.get(m.start).is_color_piece_include_king(!info.color))
                .map(|m| flatten(m.end))
                .collect(),
        ))
    } else {
        None
    };
    Ok(Html(
        TEMPLATES
            .get_chessboard(
                format!("/match/{}/{}", id, userstr),
                &coordstr,
                &board,
                info.result,
                moves,
            )
            .unwrap(),
    ))
}

#[get("/match/<id>/<userstr>/<fromstr>/to/<tostr>/promote")]
fn promotion_get(
    req: RequestWrap,
    id: u32,
    userstr: String,
    fromstr: String,
    tostr: String,
) -> Html<String> {
    Html(
        TEMPLATES
            .get_promote(format!(
                "/match/{}/{}/{}/to/{}/promote",
                id, userstr, fromstr, tostr
            ))
            .unwrap(),
    )
}

#[get("/match/<id>/<userstr>/<fromstr>/to/<tostr>/promote/<piece>")]
fn promotion_push(
    req: RequestWrap,
    id: u32,
    userstr: String,
    fromstr: String,
    tostr: String,
    piece: String,
) -> Result<Redirect, Redirect> {
    let rb = || {
        Redirect::to(format!(
            "/match/{}/{}/{}/to/{}/promote",
            id, userstr, fromstr, tostr
        ))
    };
    let piece = match piece.as_str() {
        "queen" => Piece::Queen,
        "rook" => Piece::Rook,
        "knight" => Piece::Knight,
        "bishop" => Piece::Bishop,
        "pawn" => Piece::Pawn,
        _ => Err(rb())?,
    };
    let user = get_user(&userstr, req.0).map_err(|_| rb())?;
    let reg = SyncLazy::force(&MATCH_REGISTRY);
    let info = reg.get_info(id).ok_or_else(rb)?;
    if color_to_move(user, &info) {
        let mut board = reg.get_board(id).ok_or_else(rb)?;
        let from = parse_coord(&fromstr).ok_or_else(rb)?;
        let to = parse_coord(&tostr).ok_or_else(rb)?;
        let mut moves = board.enumerate_moves(info.color, from);
        moves.filter(|mv| {
            mv.end == to
                && (if let MoveType::Promote(p, _) = mv.move_type {
                    p == piece
                } else {
                    false
                })
        });
        let mv = match moves.slice() {
            [] => Err(rb())?,
            &[mv] => mv,
            multi => multi[0],
        };
        reg.do_move(
            id,
            mv,
            (!info.extra.white_human && info.color == Color::Black)
                || (!info.extra.black_human && info.color == Color::White),
        );
        Ok(Redirect::to(format!("/match/{}/{}", id, userstr)))
    } else {
        Err(rb())
    }
}

#[get("/match/<id>/<userstr>/<fromstr>/to/<tostr>")]
fn make_move(
    req: RequestWrap,
    id: u32,
    userstr: String,
    fromstr: String,
    tostr: String,
) -> Result<Redirect, NotFound<Html<String>>> {
    let user = get_user(&userstr, req.0)?;
    let reg = SyncLazy::force(&MATCH_REGISTRY);
    let info = reg.get_info(id).ok_or_else(|| not_found(&req.0))?;
    if color_to_move(user, &info) {
        let mut board = reg.get_board(id).ok_or_else(|| not_found(&req.0))?;
        let from = parse_coord(&fromstr).ok_or_else(|| not_found(&req.0))?;
        let to = parse_coord(&tostr).ok_or_else(|| not_found(&req.0))?;
        let mut moves = board.enumerate_moves(info.color, from);
        moves.filter(|mv| mv.end == to);
        let mv = match moves.slice() {
            [] => return Err(not_found(&req.0)),
            &[mv] => mv,
            multi => {
                return if multi
                    .iter()
                    .all(|mv| matches!(mv.move_type, MoveType::Promote(_, _)))
                {
                    Ok(Redirect::to(format!(
                        "/match/{}/{}/{}/to/{}/promote",
                        id, userstr, fromstr, tostr
                    )))
                } else {
                    Err(not_found(&req.0))
                }
            }
        };
        reg.do_move(
            id,
            mv,
            (!info.extra.white_human && info.color == Color::Black)
                || (!info.extra.black_human && info.color == Color::White),
        );
        Ok(Redirect::to(format!("/match/{}/{}", id, userstr)))
    } else {
        Err(not_found(req.0))
    }
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
            .mount(
                &config.root,
                routes![
                    index,
                    new_match,
                    view_match,
                    view_match_select,
                    favicon,
                    make_move,
                    promotion_get,
                    promotion_push
                ],
            )
            .launch(),
    )
}
