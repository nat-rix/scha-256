use crate::Error;
use engine::board::{Field, Piece};
use engine::chessmatch::MatchResult;
use std::lazy::SyncLazy;

pub(crate) static TEMPLATES: SyncLazy<Templates> = SyncLazy::new(|| Templates::new().unwrap());

pub struct Templates {
    base: liquid::Template,
    index: liquid::Template,
    chessboard: liquid::Template,
    promote: liquid::Template,
    s404: liquid::Template,
    s500: liquid::Template,
}

impl Templates {
    pub fn new() -> Result<Self, Error> {
        let err = |err| Error::TemplateParsingError(Box::new(err));
        let parser = liquid::ParserBuilder::with_stdlib().build().unwrap();
        Ok(Self {
            base: parser.parse(include_str!("base.html")).map_err(err)?,
            index: parser.parse(include_str!("index.html")).map_err(err)?,
            chessboard: parser.parse(include_str!("chessboard.html")).map_err(err)?,
            promote: parser.parse(include_str!("promote.html")).map_err(err)?,
            s404: parser.parse(include_str!("404.html")).map_err(err)?,
            s500: parser.parse(include_str!("500.html")).map_err(err)?,
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

    pub fn get_chessboard(
        &self,
        rooturi: String,
        appendix: &str,
        board: &engine::board::Board,
        result: Option<MatchResult>,
        moves: Option<(i8, Vec<i8>)>,
    ) -> Result<String, Error> {
        let matrix: Vec<_> = (0..64)
            .map(|i| {
                let f = board.get(engine::board::Coord::from_xy(i % 8, i / 8));
                (match f {
                    Field::BlackKing => 1,
                    Field::BlackPiece(_) => 1,
                    _ => 0,
                }) | (match f {
                    Field::BlackKing | Field::WhiteKing => 2,
                    Field::BlackPiece(piece) | Field::WhitePiece(piece) => match piece {
                        Piece::Queen => 4,
                        Piece::Rook => 6,
                        Piece::Bishop => 8,
                        Piece::Knight => 10,
                        Piece::Pawn => 12,
                    },
                    _ => 0,
                })
            })
            .map(Into::<liquid::model::ScalarCow>::into)
            .map(liquid::model::Value::Scalar)
            .collect();
        let mut movematrix: Vec<bool> = core::iter::repeat(false).take(64).collect();
        for &coord in moves.iter().map(|i| i.1.iter()).flatten() {
            movematrix[coord as usize] = true;
        }
        let xy = moves.map(|(v, _)| v).unwrap_or(-1);
        self.get_base(
            "Chess Match",
            self.chessboard
                .render(&liquid::object! {{
                    "matrix": liquid::model::Value::Array(matrix),
                    "rooturi": rooturi,
                    "appendix": appendix,
                    "fcoord": xy,
                    "status": match result {
                        None => 0,
                        Some(MatchResult::WhiteWins) => 1,
                        Some(MatchResult::BlackWins) => 2,
                        Some(MatchResult::Stalemate) => 3,
                    },
                    "moves": movematrix,
                }})
                .map_err(Self::parsing_err),
        )
    }

    pub fn get_promote(&self, uri: String) -> Result<String, Error> {
        self.get_base(
            "Chess Match - Promotion",
            self.promote
                .render(&liquid::object! {{"uri": uri}})
                .map_err(Self::parsing_err),
        )
    }
}
