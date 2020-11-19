use crate::board::{Board, Color, Coord, Field, Piece};
use crate::moves::{LongMoveList, Move, MoveType, PromotionType};
use crate::score::{self, Score};

pub const DEFAULT_CONFIG: Config = Config {
    depth: 5,
    max_quiescence_depth: 4,
};

#[derive(Clone, Debug)]
pub struct Config {
    pub depth: u32,
    pub max_quiescence_depth: u32,
}

pub fn decide(board: &Board, color: Color, config: Config) -> Option<Move> {
    let now = std::time::Instant::now();
    let moves = get_sorted_moves(board, color);
    max_stage(
        board,
        &moves,
        config.depth,
        config.max_quiescence_depth,
        Score::min(),
        [Score::min(), Score::max()],
        color,
    )
    .map(|(m, s)| {
        println!(
            "Completed decision with a final score of {:?}. The computation took {:?}",
            s,
            now.elapsed()
        );
        m
    })
}

fn get_move_score(board: &Board, mv: &Move) -> i32 {
    let s = |f| match board.get(f) {
        Field::BlackPiece(p) | Field::WhitePiece(p) => Score::value_from_piece(*p),
        _ => 0,
    };
    match mv.move_type {
        MoveType::Capture => s(mv.end) - (s(mv.start) >> 2),
        MoveType::Castle(_) => score::CASTLING_MOVE_SCORE,
        MoveType::Promote(p, t) => {
            Score::value_from_piece(p)
                + match t {
                    PromotionType::Regular => 0,
                    PromotionType::Capture => s(mv.end),
                }
        }
        MoveType::EnPassant(_) => Score::value_from_piece(Piece::Pawn),
        _ => 0,
    }
}

fn get_white_board_score(board: &Board) -> i32 {
    let mut n = 21;
    let mut s = 0;
    for _ in 0..8 {
        for _ in 0..8 {
            let coord = unsafe { Coord::new_unchecked(n) };
            let f = board.get(coord);
            s += match f {
                Field::WhitePiece(p) => Score::value_from_piece(*p),
                Field::BlackPiece(p) => -Score::value_from_piece(*p),
                _ => 0,
            };
            let bounty = Score::threat_bounty(*f);
            for &threat in board.threat_mask.get(coord).slice() {
                s += match board.get(threat) {
                    Field::WhitePiece(_) | Field::WhiteKing => bounty,
                    Field::BlackPiece(_) | Field::BlackKing => -bounty,
                    _ => 0,
                }
            }
            let x = coord.get_inline_x();
            s += unsafe { score::CENTRAL_PIECE_AWARDENING.get_unchecked(x as usize) };
            if x == 1 || x == 8 {
                match *f {
                    Field::WhitePiece(p) => s += Score::border_penalty(p),
                    Field::BlackPiece(p) => s -= Score::border_penalty(p),
                    _ => (),
                }
            }
            n += 1
        }
        n += 2;
    }
    s
}

fn get_sorted_moves(board: &Board, color: Color) -> LongMoveList {
    let mut lst = LongMoveList::new();
    board.enumerate_all_moves_by(color, &mut lst);
    // TODO: make benchmark to test if `sort_by_cached_key` is faster
    lst.slice_mut()
        .sort_unstable_by_key(|m| -get_move_score(&board, m));
    lst
}

fn stage_common<
    F: Fn(&Board, &LongMoveList, u32, u32, Score, [Score; 2], Color) -> Option<(Move, Score)>,
>(
    board: &Board,
    mv: Move,
    color: Color,
    d: u32,
    q: u32,
    mut last_score: Score,
    win: [Score; 2],
    pv_found: bool,
    nonescore: Score,
    f: F,
) -> Score {
    let mut board = board.clone();
    board.do_move(mv);
    board.update_aggressors(!color);
    let sc = || {
        let score = get_white_board_score(&board);
        Score::Value(if (Color::White == color) == (nonescore == Score::MeWins) {
            score
        } else {
            -score
        })
    };
    let ccs = |d, q, last_score| {
        let moves = get_sorted_moves(&board, !color);
        let cs = |win| {
            f(&board, &moves, d, q, last_score, win, !color)
                .map(|(_, s)| s)
                .unwrap_or_else(|| {
                    if board.get_king(!color).aggressors.is_empty() {
                        Score::Stalemate
                    } else {
                        nonescore
                    }
                })
        };
        if pv_found {
            let score = cs([win[0], win[0] + 1]);
            if score > win[0] && score < win[1] {
                cs([score, win[1]])
            } else {
                score
            }
        } else {
            cs(win)
        }
    };
    if d > 0 {
        if d == 1 {
            last_score = sc()
        }
        ccs(d - 1, q, last_score)
    } else {
        let now = sc();
        if q == 0 || now.is_in_quiescence_bounds(last_score) {
            now
        } else {
            ccs(0, q - 1, now)
        }
    }
}

fn min_stage(
    board: &Board,
    moves: &LongMoveList,
    d: u32,
    q: u32,
    last_score: Score,
    mut win: [Score; 2],
    color: Color,
) -> Option<(Move, Score)> {
    let mut min_score = None;
    let mut pv_found = false;
    for &mv in moves.slice() {
        let score = stage_common(
            &board,
            mv,
            color,
            d,
            q,
            last_score,
            win,
            pv_found,
            Score::EnemyWins,
            max_stage,
        );
        if min_score.map_or_else(|| true, |(_, s)| score < s) {
            min_score = Some((mv, score));
            if score <= win[0] {
                break;
            }
        }
        if score < win[1] {
            win[1] = score;
            pv_found = true;
        }
    }
    min_score
}

fn max_stage(
    board: &Board,
    moves: &LongMoveList,
    d: u32,
    q: u32,
    last_score: Score,
    mut win: [Score; 2],
    color: Color,
) -> Option<(Move, Score)> {
    let mut max_score = None;
    let mut pv_found = false;
    for &mv in moves.slice() {
        let score = stage_common(
            &board,
            mv,
            color,
            d,
            q,
            last_score,
            win,
            pv_found,
            Score::MeWins,
            min_stage,
        );
        if max_score.map_or_else(|| true, |(_, s)| score > s) {
            max_score = Some((mv, score));
            if score >= win[1] {
                break;
            }
        }
        if score > win[0] {
            win[0] = score;
            pv_found = true;
        }
    }
    max_score
}
