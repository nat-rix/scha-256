#![allow(incomplete_features)]
#![feature(const_generics)]
#![feature(array_value_iter)]
#![feature(maybe_uninit_ref)]

pub mod board;
pub mod check;
pub mod list;
pub mod moves;
mod web;

use std::io::{BufRead, Write};

fn get_line(
    xoff: usize,
    lines: &mut std::io::Lines<std::io::StdinLock>,
    retry: bool,
) -> Option<board::Coord> {
    loop {
        print!("\x1b[{}C\x1b[0;1m> ", xoff);
        std::io::stdout().flush().ok()?;
        break match lines.next() {
            Some(Ok(line)) => match line.parse::<board::Coord>() {
                Ok(coord) => Some(coord),
                Err(err) => {
                    println!("\x1b[{}C\x1b[3;91m{}\x1b[m", xoff, err);
                    if retry {
                        continue;
                    } else {
                        return None;
                    }
                }
            },
            _ => None,
        };
    }
}

fn main() {
    crate::web::run_webserver(board::Board::new());

    /*
    let mut board = board::Board::new();
    let stdin = std::io::stdin();
    let mut lines = stdin.lock().lines();

    let mut color = board::Color::White;
    let mut clear_screen = true;

    'outerloop: loop {
        let xoff = 48;
        if clear_screen {
            print!("\x1b[2J\x1bc");
            board.output_terminal(true, &[]);
            println!("\x1b[1;{}H\x1b[m >>> \x1b[1;38;5;163mS \x1b[38;5;226mC \x1b[38;5;196mH \x1b[38;5;75mA \x1b[38;5;215mC \x1b[38;5;40mH \x1b[m<<<\n", xoff);
            match color {
                board::Color::White => {
                    println!("\x1b[{}C\x1b[0;1;30;107m WHITE \x1b[m to move\n", xoff)
                }
                board::Color::Black => println!(
                    "\x1b[{}C\x1b[0;1;48;5;233;38;5;246m BLACK \x1b[m to move\n",
                    xoff
                ),
            }
        } else {
            clear_screen = true;
        }
        while let Some(coord) = get_line(xoff, &mut lines, false) {
            print!("\x1b[s\x1b7\x1b[1;1H");
            board.output_terminal(true, board.threat_mask.get(coord));
            print!("\x1b[u\x1b8");
        }
        let (moves, hl) = loop {
            let coord = match get_line(xoff, &mut lines, true) {
                Some(v) => v,
                _ => break 'outerloop,
            };
            if board.get(coord).is_color_piece_include_king(!color) {
                let moves = board.enumerate_moves(coord);
                let hl: Vec<_> = moves.slice().iter().map(|m| m.end).collect();
                if hl.is_empty() {
                    println!(
                        "\x1b[{}C\x1b[3;91mthis piece has no valid moves\x1b[m",
                        xoff
                    );
                    continue;
                }
                break (moves, hl);
            }
            println!("\x1b[{}C\x1b[3;91myou cannot move that piece\x1b[m", xoff);
        };
        print!("\x1b[s\x1b7\x1b[1;1H");
        board.output_terminal(true, &hl);
        print!("\x1b[u\x1b8");
        let coord = match get_line(xoff, &mut lines, true) {
            Some(v) => v,
            _ => break,
        };
        if let Some(&m) = moves.slice().iter().find(|m| m.end == coord) {
            if let moves::MoveType::Promote(_, promotion_type) = m.move_type {
                let piece = loop {
                    println!(
                        "\x1b[{}C\x1b[mChoose between | 1) ♛  | 2) ♜ | 3) ♝ | 4) ♞ | 5) ♟ |\x1b[m",
                        xoff
                    );
                    let line = lines.next().map(Result::ok).flatten();
                    break match line.as_ref().map(|s| s.trim()) {
                        Some("1") => board::Piece::Queen,
                        Some("2") => board::Piece::Rook,
                        Some("3") => board::Piece::Bishop,
                        Some("4") => board::Piece::Knight,
                        Some("5") => board::Piece::Pawn,
                        Some(_) => {
                            println!("\x1b[{}C\x1b[3;91minvalid character, expecting integer in range 1 to 5\x1b[m", xoff);
                            continue;
                        }
                        None => break 'outerloop,
                    };
                };
                let mut m = m;
                m.move_type = moves::MoveType::Promote(piece, promotion_type);
                board.do_move(m);
            } else {
                board.do_move(m);
            }
            color = !color;
        } else {
            println!("\x1b[{}C\x1b[3;91minvalid move\x1b[m", xoff);
            print!("\x1b[s\x1b7\x1b[1;1H");
            board.output_terminal(true, &[]);
            print!("\x1b[u\x1b8");
            clear_screen = false;
        }
    }*/
}
