use crate::board::{Board, Color, Coord, Field, Piece};
use core::cmp::Ordering;

const QUEEN_VALUE: i32 = 950;
const ROOK_VALUE: i32 = 563;
const BISHOP_VALUE: i32 = 333;
const KNIGHT_VALUE: i32 = 305;
const PAWN_VALUE: i32 = 100;

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
