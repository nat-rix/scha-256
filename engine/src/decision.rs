use crate::board::{Board, Color, Coord, Field, Piece};
use crate::moves::{LongMoveList, Move};
use crate::score::Score;

pub const DEFAULT_CONFIG: Config = Config { depth: 6 };

#[derive(Clone, Debug)]
pub struct Config {
    pub depth: u32,
}

pub fn decide(board: &Board, color: Color, config: Config) -> Option<Move> {
    let moves = get_sorted_moves(board, color);
    max_stage(
        board,
        moves,
        config.depth,
        [Score::min(), Score::max()],
        color,
    )
    .map(|(m, s)| m)
}

fn get_move_score(board: &Board, mv: &Move) -> i32 {
    let s = |f| match board.get(f) {
        Field::BlackPiece(p) | Field::WhitePiece(p) => Score::value_from_piece(*p),
        _ => 0,
    };
    s(mv.end) - (s(mv.start) >> 1)
}

fn get_white_board_score(board: &Board) -> i32 {
    let mut n = 21;
    let mut s = 0;
    for _ in 0..8 {
        for _ in 0..8 {
            let f = board.get(unsafe { Coord::new_unchecked(n) });
            s += match f {
                Field::WhitePiece(p) => Score::value_from_piece(*p),
                Field::BlackPiece(p) => -Score::value_from_piece(*p),
                _ => 0,
            };
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
        .sort_unstable_by_key(|m| get_move_score(&board, m));
    lst
}

fn stage_common<F: Fn(&Board, LongMoveList, u32, [Score; 2], Color) -> Option<(Move, Score)>>(
    board: &Board,
    mv: Move,
    color: Color,
    d: u32,
    win: [Score; 2],
    nonescore: Score,
    f: F,
) -> Score {
    let mut board = board.clone();
    board.do_move(mv);
    board.update_aggressors(!color);
    if d > 0 {
        let moves = get_sorted_moves(&board, !color);
        f(&board, moves, d - 1, win, !color)
            .map(|(_, s)| s)
            .unwrap_or_else(|| {
                if board.get_king(!color).aggressors.is_empty() {
                    Score::Stalemate
                } else {
                    nonescore
                }
            })
    } else {
        let score = get_white_board_score(&board);
        Score::Value(if (Color::White == color) == (nonescore == Score::MeWins) {
            score
        } else {
            -score
        })
    }
}

fn min_stage(
    board: &Board,
    moves: LongMoveList,
    d: u32,
    mut win: [Score; 2],
    color: Color,
) -> Option<(Move, Score)> {
    let mut min_score = None;
    for &mv in moves.slice() {
        let score = stage_common(&board, mv, color, d, win, Score::EnemyWins, max_stage);
        if min_score.map_or_else(|| true, |(_, s)| score < s) {
            min_score = Some((mv, score));
            win[1] = score;
        }
        if score <= win[0] {
            break;
        }
    }
    min_score
}

fn max_stage(
    board: &Board,
    moves: LongMoveList,
    d: u32,
    mut win: [Score; 2],
    color: Color,
) -> Option<(Move, Score)> {
    let mut max_score = None;
    for &mv in moves.slice() {
        let score = stage_common(&board, mv, color, d, win, Score::MeWins, min_stage);
        if max_score.map_or_else(|| true, |(_, s)| score > s) {
            max_score = Some((mv, score));
            win[0] = score;
        }
        if score >= win[1] {
            break;
        }
    }
    max_score
}
