use crate::board::{Field, Piece};
use core::cmp::Ordering;

pub const QUEEN_VALUE: i32 = 9500;
pub const ROOK_VALUE: i32 = 5630;
pub const BISHOP_VALUE: i32 = 3330;
pub const KNIGHT_VALUE: i32 = 3050;
pub const PAWN_VALUE: i32 = 1000;

pub const QUEEN_THREATENED_VALUE: i32 = 126;
pub const ROOK_THREATENED_VALUE: i32 = 96;
pub const BISHOP_THREATENED_VALUE: i32 = 84;
pub const KNIGHT_THREATENED_VALUE: i32 = 80;
pub const PAWN_THREATENED_VALUE: i32 = 71;
pub const EMPTY_THREATENED_VALUE: i32 = 54;

pub const CENTRAL_PIECE_AWARDENING: [i32; 10] = [-100, 0, 200, 300, 300, 200, 0, -100, 0, 0];
pub const BORDER_QUEEN_PENALTY: i32 = -600;
pub const BORDER_ROOK_PENALTY: i32 = -400;
pub const BORDER_BISHOP_PENALTY: i32 = -600;
pub const BORDER_KNIGHT_PENALTY: i32 = -1000;
pub const BORDER_PAWN_PENALTY: i32 = 0;

pub const CASTLING_MOVE_SCORE: i32 = 900;

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

    pub const fn max() -> Self {
        Self::MeWins
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

    pub const fn threat_bounty(f: Field) -> i32 {
        match f {
            Field::Empty => EMPTY_THREATENED_VALUE,
            Field::BlackPiece(p) | Field::WhitePiece(p) => match p {
                Piece::Queen => QUEEN_THREATENED_VALUE,
                Piece::Rook => ROOK_THREATENED_VALUE,
                Piece::Bishop => BISHOP_THREATENED_VALUE,
                Piece::Knight => KNIGHT_THREATENED_VALUE,
                Piece::Pawn => PAWN_THREATENED_VALUE,
            },
            _ => 0,
        }
    }

    pub const fn border_penalty(p: Piece) -> i32 {
        match p {
            Piece::Queen => BORDER_QUEEN_PENALTY,
            Piece::Rook => BORDER_ROOK_PENALTY,
            Piece::Bishop => BORDER_BISHOP_PENALTY,
            Piece::Knight => BORDER_KNIGHT_PENALTY,
            Piece::Pawn => BORDER_PAWN_PENALTY,
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
