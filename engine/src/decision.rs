use crate::board::{Board, Color, Coord, Field, Piece};
use crate::moves::{LongMoveList, Move, MoveType, PromotionType};
use crate::score::{self, Score};

pub const DEFAULT_CONFIG: Config = Config { depth: 5 };

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
    .map(|(m, _)| m)
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
            let mass = match f {
                Field::WhitePiece(p) => Score::value_from_piece(*p),
                Field::BlackPiece(p) => -Score::value_from_piece(*p),
                _ => 0,
            };
            let bounty = Score::threat_bounty(*f);
            let bounty_awards: i32 = board
                .threat_mask
                .get(coord)
                .slice()
                .iter()
                .map(|&t| match board.get(t) {
                    Field::WhitePiece(_) | Field::WhiteKing => bounty,
                    Field::BlackPiece(_) | Field::BlackKing => -bounty,
                    _ => 0,
                })
                .sum();
            let (x, _y) = coord.as_xy();
            let central_positioning_award = score::CENTRAL_PIECE_AWARDENING[x as usize];
            let border_penalty = if x == 0 || x == 7 {
                match *f {
                    Field::WhitePiece(p) => Score::border_penalty(p),
                    Field::BlackPiece(p) => -Score::border_penalty(p),
                    _ => 0,
                }
            } else {
                0
            };
            s += mass + bounty_awards + central_positioning_award + border_penalty;
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
