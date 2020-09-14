use crate::board::{Board, Color, Coord, Piece};
use crate::moves::MoveType;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

struct HttpLineReader<T: Read + Write>(T);

#[derive(Debug, Clone)]
struct UiStatus {
    color: Color,
}

impl<T: Read + Write> HttpLineReader<T> {
    fn push(&mut self, val: &[u8]) -> std::io::Result<()> {
        self.0.write_all(val)
    }
}

impl<T: Read + Write> Iterator for HttpLineReader<T> {
    type Item = std::io::Result<String>;
    fn next(&mut self) -> Option<Self::Item> {
        let mut buf = vec![];
        while !buf.ends_with(b"\r\n") {
            let mut byte = [0];
            if let Err(e) = self.0.read_exact(&mut byte) {
                return Some(Err(e));
            }
            buf.push(byte[0]);
        }
        buf.truncate(buf.len() - 2);
        if let Ok(buf) = String::from_utf8(buf) {
            Some(Ok(buf))
        } else {
            Some(Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "invalid utf-8",
            )))
        }
    }
}

#[derive(Debug)]
enum HttpError {
    UnexpectedEndOfHeader,
    UnexpectedEndOfLine,
    InvalidRequestMethod,
    IoError(std::io::Error),
}

#[derive(Debug, Clone)]
enum HttpMethod {
    Get,
    Post,
}

fn read_request<T: Read + Write>(
    lines: &mut HttpLineReader<T>,
) -> Result<(HttpMethod, String), HttpError> {
    let next = lines
        .next()
        .ok_or(HttpError::UnexpectedEndOfHeader)?
        .map_err(HttpError::IoError)?;
    let mut first_row = next.split(' ');
    let method = match first_row.next().ok_or(HttpError::UnexpectedEndOfLine)? {
        "GET" => HttpMethod::Get,
        "POST" => HttpMethod::Post,
        _ => return Err(HttpError::InvalidRequestMethod),
    };
    let location = first_row.next().ok_or(HttpError::UnexpectedEndOfLine)?;
    let _http_ver = first_row.next().ok_or(HttpError::UnexpectedEndOfLine)?;
    for line in lines {
        if line.map_err(HttpError::IoError)?.is_empty() {
            break;
        }
    }
    Ok((method, location.to_string()))
}

fn unicode_to_html(chr: char) -> String {
    chr.escape_unicode()
        .skip(3)
        .take_while(|&c| c != '}')
        .collect()
}

fn response<T: Read + Write>(
    writer: &mut HttpLineReader<T>,
    board: &mut Board,
    from: Option<(i8, i8)>,
    hl: &[Coord],
    status: &UiStatus,
    is_inspect_threat: bool,
) -> std::io::Result<()> {
    let mut send: Vec<u8> = vec![];
    writeln!(&mut send, "<!doctype html>")?;
    writeln!(&mut send, "<html>")?;
    writeln!(&mut send, "  <head><title>Schach</title></head>")?;
    writeln!(&mut send, "  <body>")?;
    writeln!(
        &mut send,
        "    <div class='row menu-bar' style='justify-content: space-between;'>"
    )?;
    writeln!(
        &mut send,
        "      <a href='/' style='text-decoration: none;'><button>&#x1f5f6; abort</button></a>"
    )?;
    if !is_inspect_threat {
        writeln!(
            &mut send,
            "      <a href='/inspect-threat' style='text-decoration: none;'><button>inspect threats</button></a>"
        )?;
    }
    let colorstr = match status.color {
        Color::White => "white",
        Color::Black => "black",
    };
    writeln!(&mut send, "    <div style='display: flex; column-gap: 0.32em; align-items: center; font-size: 1.5em;'><div style='width: 1em; height: 1em; border: solid 2px black; border-radius: 1em; background: {};'></div><span style='transform: translateY(4px)'> <b>{}</b> to move</span></div>", colorstr, colorstr.to_uppercase())?;
    writeln!(&mut send, "    </div>")?;
    writeln!(&mut send, "    <div class='board'>")?;
    for y in 0..8 {
        writeln!(&mut send, "      <div class='row'>")?;
        for x in 0..8 {
            let coord = Coord::from_xy(x, y);
            let (color, chr) = board.get(coord).repr();
            write!(&mut send, "        <a href='")?;
            if is_inspect_threat {
                write!(&mut send, "/inspect-threat/{}/{}'", x, y)?;
            } else if let Some((from_x, from_y)) = from {
                write!(&mut send, "/from/{}/{}/to/{}/{}'", from_x, from_y, x, y)?;
            } else {
                write!(&mut send, "/select/{}/{}'", x, y)?;
            }
            if hl.contains(&coord) {
                write!(&mut send, " style='box-shadow: inset 0 0 0px 15px red;'")?;
            }
            if Some((x, y)) == from {
                write!(&mut send, " style='box-shadow: inset 0 0 0px 15px blue;'")?;
            }
            writeln!(
                &mut send,
                " class='cell {}'>&#x{};</a>",
                match color {
                    Color::Black => "black-cell",
                    Color::White => "white-cell",
                },
                unicode_to_html(chr)
            )?;
        }
        writeln!(&mut send, "      </div>")?;
    }
    writeln!(&mut send, "    </div>")?;
    writeln!(&mut send, "  </body>")?;
    writeln!(&mut send, "  <style>")?;
    writeln!(&mut send, "body {{ padding: 0; margin: 0; width: 100%; display: flex; flex-direction: column; align-items: center; }}")?;
    writeln!(
        &mut send,
        ".menu-bar {{ justify-content: center; padding: 1em; border-bottom: solid 1px black; margin-bottom: 1em; width: 60em; }}"
    )?;
    writeln!(
        &mut send,
        ".board {{ display: flex; flex: auto; flex-direction: column-reverse; align-self: stretch; border: solid 10px #823; width: max-content; margin-left: auto; margin-right: auto; }}"
    )?;
    writeln!(
        &mut send,
        ".row {{ display: flex; flex: auto; flex-direction: row; }}"
    )?;
    writeln!(
        &mut send,
        ".cell {{ text-decoration: none; width: 100%; text-align: center; font-size: 100px; width: 100px; height: 100px; }}"
    )?;
    writeln!(
        &mut send,
        ".row:nth-child(even) .cell:nth-child(even) {{ background: black; }}"
    )?;
    writeln!(
        &mut send,
        ".row:nth-child(odd) .cell:nth-child(odd) {{ background: black; }}"
    )?;
    writeln!(
        &mut send,
        ".white-cell {{ color: white; text-shadow: 0 0 1px black, 0 0 2px black; }}"
    )?;
    writeln!(
        &mut send,
        ".black-cell {{ color: black; text-shadow: 0 0 1px white, 0 0 2px white; }}"
    )?;
    writeln!(&mut send, "  </style>")?;
    writeln!(&mut send, "</html>")?;
    let mut allsend: Vec<u8> = vec![];
    write!(
        &mut allsend,
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
        send.len(),
        core::str::from_utf8(&send).unwrap()
    )?;
    writer.push(&allsend)
}

fn redirect<T: Read + Write>(
    writer: &mut HttpLineReader<T>,
    location: &str,
) -> std::io::Result<()> {
    writer.push(
        format!(
            "HTTP/1.1 303 See Other\r\nLocation: {}\r\nContent-Length: 0\r\n\r\n",
            location
        )
        .as_bytes(),
    )
}

fn handle_promotion<T: Read + Write>(
    writer: &mut HttpLineReader<T>,
    coords: [i8; 4],
) -> std::io::Result<()> {
    let mut send: Vec<u8> = vec![];
    write!(
        &mut send,
        "<!doctype html><html><body><h1>Choose the chess piece to promote</h1>"
    )?;
    for (u, n) in &[
        ("&#9823;", "pawn"),
        ("&#9822;", "knight"),
        ("&#9821;", "bishop"),
        ("&#9820;", "rook"),
        ("&#9819;", "queen"),
    ] {
        write!(&mut send, "<a href='/promote/{}/{}/{}/{}/{}' style='text-decoration: none; color: black;'><button style='font-size: 6em;'>{}</button></a>", n, coords[0], coords[1], coords[2], coords[3], u)?;
    }
    writeln!(&mut send, "</body></html>")?;
    let mut sendall: Vec<u8> = b"HTTP/1.1 200 OK\r\nContent-Length: ".to_vec();
    write!(&mut sendall, "{}", send.len())?;
    sendall.append(&mut b"\r\n\r\n".to_vec());
    sendall.append(&mut send);
    writer.push(&sendall)
}

fn handle_move<T: Read + Write>(
    writer: &mut HttpLineReader<T>,
    c: [&str; 4],
    board: &mut Board,
    color: &mut Color,
    promote: Option<Piece>,
) -> std::io::Result<()> {
    let mut loc = "/".to_string();
    if let (Ok(fx), Ok(fy), Ok(tx), Ok(ty)) =
        (c[0].parse(), c[1].parse(), c[2].parse(), c[3].parse())
    {
        let to = Coord::from_xy(tx, ty);
        if let Some(&mv) = board
            .enumerate_moves(Coord::from_xy(fx, fy))
            .slice()
            .iter()
            .filter(|m| board.get(m.start).is_color_piece_include_king(!*color))
            .find(|m| m.end == to)
        {
            let mut mv = mv;
            match match (mv.move_type, promote) {
                (MoveType::Promote(_, _), None) => None,
                (MoveType::Promote(_, promotion_type), Some(piece)) => {
                    mv.move_type = MoveType::Promote(piece, promotion_type);
                    Some(mv)
                }
                _ => Some(mv),
            } {
                Some(mv) => {
                    board.do_move(mv);
                    println!("{:?} made move: {:?}", *color, mv);
                    *color = !*color;
                }
                None => loc = format!("/promotion/{}/{}/{}/{}", fx, fy, tx, ty),
            };
        }
    }
    redirect(writer, &loc)
}

fn handle_get<T: Read + Write>(
    writer: &mut HttpLineReader<T>,
    location: String,
    board: &mut Arc<Mutex<(Board, UiStatus)>>,
    select: &mut Option<Coord>,
) {
    let (ref mut board, ref mut status) = &mut *board.lock().unwrap();
    let color = &mut status.color;
    match location.split('/').collect::<Vec<_>>().as_slice() {
        ["", ""] => response(writer, board, None, &[], status, false),
        ["", "select", x, y] => {
            if let (Ok(x), Ok(y)) = (x.parse(), y.parse()) {
                let hl = &board
                    .enumerate_moves(Coord::from_xy(x, y))
                    .slice()
                    .iter()
                    .map(|m| m.end)
                    .collect::<Vec<_>>();
                response(writer, board, Some((x, y)), hl, status, false)
            } else {
                Ok(())
            }
        }
        ["", "from", fx, fy, "to", tx, ty] => {
            handle_move(writer, [fx, fy, tx, ty], board, color, None)
        }
        ["", "promotion", fx, fy, tx, ty] => {
            if let (Ok(fx), Ok(fy), Ok(tx), Ok(ty)) =
                (fx.parse(), fy.parse(), tx.parse(), ty.parse())
            {
                handle_promotion(writer, [fx, fy, tx, ty])
            } else {
                Ok(())
            }
        }
        ["", "promote", piece, fx, fy, tx, ty] => {
            let piece = match *piece {
                "pawn" => Some(Piece::Pawn),
                "knight" => Some(Piece::Knight),
                "bishop" => Some(Piece::Bishop),
                "rook" => Some(Piece::Rook),
                "queen" => Some(Piece::Queen),
                _ => None,
            };
            handle_move(writer, [fx, fy, tx, ty], board, color, piece)
        }
        ["", "inspect-threat"] => response(writer, board, None, &[], status, true),
        ["", "inspect-threat", x, y] => {
            if let (Ok(x), Ok(y)) = (x.parse(), y.parse()) {
                let hl = board.threat_mask.get(Coord::from_xy(x, y)).slice().to_vec();
                response(writer, board, Some((x, y)), &hl, status, true)
            } else {
                Ok(())
            }
        }
        _ => Ok(()),
    }
    .unwrap();
}

fn threaded_client(stream: TcpStream, mut board: Arc<Mutex<(Board, UiStatus)>>) {
    let mut lines = HttpLineReader(stream);
    let mut select = None;
    loop {
        match read_request(&mut lines) {
            Ok((HttpMethod::Get, location)) => {
                handle_get(&mut lines, location, &mut board, &mut select)
            }
            Ok((method, location)) => println!(
                "error invalid request method '{:?}' at '{}'",
                method, location
            ),
            Err(HttpError::UnexpectedEndOfLine)
            | Err(HttpError::UnexpectedEndOfHeader)
            | Err(HttpError::IoError(_)) => break,
            Err(e) => println!("error parsing request: {:?}", e),
        }
    }
}

pub fn run_webserver(board: Board) {
    let status = UiStatus {
        color: Color::White,
    };
    let glob_board = Arc::new(Mutex::new((board, status)));
    let listener = TcpListener::bind("127.0.0.1:3999").unwrap();
    for con in listener.incoming() {
        let con = match con {
            Ok(con) => con,
            Err(e) => {
                println!("error: client connection failed '{}'", e);
                continue;
            }
        };
        let board_ref = Arc::clone(&glob_board);
        std::thread::spawn(|| threaded_client(con, board_ref));
    }
}
