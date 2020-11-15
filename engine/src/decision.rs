use crate::board::{Board, Color, Coord, Field, Piece};
use crate::moves::{LongMoveList, Move};
use core::cmp::Ordering;

pub type DefaultDecisionMaker = DecisionMaker<3>;

const QUEEN_VALUE: i32 = 950;
const ROOK_VALUE: i32 = 563;
const BISHOP_VALUE: i32 = 333;
const KNIGHT_VALUE: i32 = 305;
const PAWN_VALUE: i32 = 100;

const MAX_VALUE_DELTA_UP: i32 = 35;
const MAX_VALUE_DELTA_DOWN: i32 = 35;

#[derive(Debug, Clone)]
pub struct DecisionMaker<const D: usize> {
    board: Board,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Score {
    MeWins,
    EnemyWins,
    Stalemate,
    Value(i32),
}

impl Score {
    pub const fn min() -> Self {
        Self::EnemyWins
    }

    pub const fn value_from_piece(p: Piece) -> i32 {
        match p {
            Piece::Queen => QUEEN_VALUE,
            Piece::Rook => ROOK_VALUE,
            Piece::Bishop => BISHOP_VALUE,
            Piece::Knight => KNIGHT_VALUE,
            Piece::Pawn => PAWN_VALUE,
        }
    }

    pub const fn value_from_field(color: Color, f: Field) -> i32 {
        match (color, f) {
            (Color::Black, Field::BlackPiece(p)) | (Color::White, Field::WhitePiece(p)) => {
                Self::value_from_piece(p)
            }
            (Color::White, Field::BlackPiece(p)) | (Color::Black, Field::WhitePiece(p)) => {
                -Self::value_from_piece(p)
            }
            (_, _) => 0,
        }
    }
}

impl core::ops::Add<i32> for Score {
    type Output = Self;
    fn add(self, o: i32) -> Self {
        match self {
            Self::Value(v) => Self::Value(v + o),
            v => v,
        }
    }
}

impl core::ops::Neg for Score {
    type Output = Self;
    fn neg(self) -> Self {
        match self {
            Self::MeWins => Self::EnemyWins,
            Self::EnemyWins => Self::MeWins,
            Self::Stalemate => Self::Stalemate,
            Self::Value(v) => Self::Value(-v),
        }
    }
}

impl Ord for Score {
    fn cmp(&self, other: &Self) -> Ordering {
        if self == other {
            return Ordering::Equal;
        };
        match (self, other) {
            (Self::MeWins, _) => Ordering::Greater,
            (Self::EnemyWins, _) => Ordering::Less,
            (Self::Stalemate, Self::EnemyWins) => Ordering::Greater,
            (Self::Stalemate, _) => Ordering::Less,
            (Self::Value(_), Self::MeWins) => Ordering::Less,
            (Self::Value(_), Self::Stalemate) | (Self::Value(_), Self::EnemyWins) => {
                Ordering::Greater
            }
            (Self::Value(a), Self::Value(b)) => a.cmp(b),
        }
    }
}

impl PartialOrd for Score {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<const D: usize> DecisionMaker<D> {
    pub fn from_board(board: Board) -> Self {
        Self { board }
    }

    pub fn with_board(&self, board: Board) -> Self {
        Self { board }
    }

    pub fn board_score(&self, color: Color, last_moved: Color, board: &Board) -> Score {
        // TODO: optimize for lazy evaluation
        let mut moves = LongMoveList::new();
        board.enumerate_all_moves_by(color, &mut moves);
        if moves.is_empty() {
            return if board.get_king(color).aggressors.is_empty() {
                Score::Stalemate
            } else if color == last_moved {
                Score::MeWins
            } else {
                Score::EnemyWins
            };
        }
        Score::Value(
            (2..10)
                .map(|v| {
                    let v10 = v * 10;
                    (v10 + 1)..(v10 + 9)
                })
                .flatten()
                .map(|n| {
                    Score::value_from_field(color, *board.get(unsafe { Coord::new_unchecked(n) }))
                })
                .sum(),
        )
    }

    fn get_and_then<T, F: Fn<I: Iterator<Item = (Board, Move, Score)>>(&mut I) -> T>(
        &self,
        color: Color,
        f: F,
    ) -> T {
        let mut moves = LongMoveList::new();
        self.board.enumerate_all_moves_by(color, &mut moves);
        f(&mut moves
            .slice()
            .iter()
            .map(|mvs| mvs.slice().iter())
            .flatten()
            .map(|&mv| {
                let mut bc = self.board.clone();
                bc.do_move(mv);
                bc.update_aggressors(color);
                let score = self.board_score(color, !color, &bc);
                (bc, mv, score)
            }))
    }

    pub fn get(&self, color: Color) -> Option<Move> {
        let score = self.board_score(color, !color, &self.board);
        let mv = self
            .get_best_move_for_me(D - 1, color, score)
            .map(|(m, _s)| m);
        if mv.is_none() {
            panic!("oh shit, no solution");
        }
        mv
    }

    fn get_worst_move_for_me(
        &self,
        d: usize,
        color: Color,
        initscore: Score,
    ) -> Option<(Move, Score)> {
        self.get_and_then(color, |i| {
            i.filter_map(|(b, m, old_s)| {
                self.with_board(b)
                    .get_best_move_for_me(d, !color, initscore)
                    .map(|(_, s)| (m, s, old_s))
            })
            /*.inspect(|(m, s, old_s)| {
                if d == D - 2 {
                    println!("conter: {:?} = {:?}/{:?}", m, s, old_s)
                }
            })*/
            .min_by_key(|(_, s, _)| *s)
            //.filter(|(_, _, s)| *s <= -initscore + MAX_VALUE_DELTA_UP)
            .map(|(m, s, _)| (m, s))
        })
    }

    fn get_best_move_for_me(
        &self,
        d: usize,
        color: Color,
        initscore: Score,
    ) -> Option<(Move, Score)> {
        self.get_and_then(color, |i| {
            i //.filter(|(_, _, s)| *s >= initscore + -MAX_VALUE_DELTA_DOWN)
                .filter_map(|(b, m, s)| {
                    if d > 0 {
                        self.with_board(b)
                            .get_worst_move_for_me(d - 1, !color, initscore)
                            .map(|(_, s)| (m, s))
                    } else {
                        Some((m, s))
                    }
                })
                /*.inspect(|(m, s)| {
                    if d == D - 1 {
                        println!("move: {:?} = {:?}", m, s)
                    }
                })*/
                .max_by_key(|(_, s)| *s)
        })
    }
}
