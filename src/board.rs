use crate::check::ThreatMask;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    White,
    Black,
}

impl core::ops::Not for Color {
    type Output = Self;
    fn not(self) -> Self {
        match self {
            Self::White => Self::Black,
            Self::Black => Self::White,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Piece {
    Queen,
    Rook,
    Bishop,
    Knight,
    Pawn,
}

impl Piece {
    pub const fn unicode(&self) -> char {
        match self {
            Piece::Queen => '♛',
            Piece::Rook => '♜',
            Piece::Bishop => '♝',
            Piece::Knight => '♞',
            Piece::Pawn => '♟',
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Empty,
    Invincible,
    BlackKing,
    WhiteKing,
    BlackPiece(Piece),
    WhitePiece(Piece),
}

impl Field {
    pub fn repr(&self) -> (Color, char) {
        match self {
            Field::WhitePiece(p) => (Color::White, p.unicode()),
            Field::BlackPiece(p) => (Color::Black, p.unicode()),
            Field::WhiteKing => (Color::White, '♚'),
            Field::BlackKing => (Color::Black, '♚'),
            _ => (Color::Black, ' '),
        }
    }

    pub fn unicode(&self) -> String {
        let (color, chr) = self.repr();
        format!(
            "\x1b[{}m{}",
            match color {
                Color::White => 97,
                Color::Black => 30,
            },
            chr
        )
    }

    pub const fn is_color_piece(&self, color: Color) -> bool {
        if let Color::Black = color {
            matches!(self, Self::WhitePiece(_))
        } else {
            matches!(self, Self::BlackPiece(_))
        }
    }

    pub const fn is_color_piece_include_king(&self, color: Color) -> bool {
        if let Color::Black = color {
            matches!(self, Self::WhitePiece(_) | Self::WhiteKing)
        } else {
            matches!(self, Self::BlackPiece(_) | Self::BlackKing)
        }
    }
}

pub trait CommonCoord {
    fn raw(&self) -> i8;
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnsafeCoord(pub(crate) i8);

impl CommonCoord for UnsafeCoord {
    fn raw(&self) -> i8 {
        self.0
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Coord(pub(crate) core::num::NonZeroI8);

impl CommonCoord for Coord {
    fn raw(&self) -> i8 {
        self.0.get()
    }
}

#[derive(Debug, Clone)]
pub enum CoordFromStrError {
    CharacterCount,
    InvalidLetter,
    InvalidNumber,
}

impl std::fmt::Display for CoordFromStrError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::CharacterCount => "expected two characters e.g. 'C3'",
                Self::InvalidLetter => "First expecting a letter e.g. 'D'",
                Self::InvalidNumber => "expecting a number as the second character e.g. '2'",
            }
        )
    }
}

impl core::str::FromStr for Coord {
    type Err = CoordFromStrError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut chars = s.trim().chars().map(char::to_lowercase).flatten();
        if let (Some(c1), Some(c2), None) = (chars.next(), chars.next(), chars.next()) {
            let x = match c1 {
                'a' => 0,
                'b' => 1,
                'c' => 2,
                'd' => 3,
                'e' => 4,
                'f' => 5,
                'g' => 6,
                'h' => 7,
                _ => return Err(CoordFromStrError::InvalidLetter),
            };
            let y = match c2 {
                '1' => 0,
                '2' => 1,
                '3' => 2,
                '4' => 3,
                '5' => 4,
                '6' => 5,
                '7' => 6,
                '8' => 7,
                _ => return Err(CoordFromStrError::InvalidNumber),
            };
            Ok(Self::from_xy(x, y))
        } else {
            Err(CoordFromStrError::CharacterCount)
        }
    }
}

impl UnsafeCoord {
    pub const fn baseline(&self) -> Option<Color> {
        match self.0 {
            31..=38 => Some(Color::White),
            81..=88 => Some(Color::Black),
            _ => None,
        }
    }

    pub const fn endline(&self) -> Option<Color> {
        match self.0 {
            21..=28 => Some(Color::Black),
            91..=98 => Some(Color::White),
            _ => None,
        }
    }

    pub const fn as_safe(self) -> Option<Coord> {
        if self.0 > 20 && self.0 < 99 {
            let mod10 = self.0 % 10;
            if mod10 == 0 || mod10 == 9 {
                Some(unsafe { Coord::new_unchecked(self.0) })
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl Coord {
    pub const fn from_xy(x: i8, y: i8) -> Self {
        let _ = ([0; 8][x as usize], [0; 8][y as usize]);
        unsafe { Self::new_unchecked(21 + x + y * 10) }
    }

    pub const fn rel(self, x: i8, y: i8) -> UnsafeCoord {
        let _ = ([0; 3][x.abs() as usize], [0; 3][y.abs() as usize]);
        UnsafeCoord(self.0.get() + x + y * 10)
    }

    /// # Safety
    /// Safe as long n = 21 + i + j * 10 with i, j
    /// being integers from 0 to 7.
    pub const unsafe fn new_unchecked(n: i8) -> Self {
        Self(core::num::NonZeroI8::new_unchecked(n))
    }

    pub const fn as_unsafe(self) -> UnsafeCoord {
        UnsafeCoord(self.0.get())
    }

    pub const fn baseline(&self) -> Option<Color> {
        self.as_unsafe().baseline()
    }

    pub const fn endline(&self) -> Option<Color> {
        self.as_unsafe().endline()
    }
}

#[derive(Clone)]
pub struct Board {
    data: [Field; 10 * 12],
    pub(crate) en_passant_chance: Option<Coord>,
    pub(crate) threat_mask: ThreatMask,
    pub(crate) restore_stack: crate::moves::RestoreStack,
}

impl Board {
    #[allow(non_snake_case)]
    pub fn new() -> Self {
        use Field::{
            BlackKing as BK, BlackPiece, Empty as EE, Invincible as II, WhiteKing as WK, WhitePiece,
        };
        use Piece::*;
        let (WR, WN, WB, WQ, WP) = (
            WhitePiece(Rook),
            WhitePiece(Knight),
            WhitePiece(Bishop),
            WhitePiece(Queen),
            WhitePiece(Pawn),
        );
        let (BR, BN, BB, BQ, BP) = (
            BlackPiece(Rook),
            BlackPiece(Knight),
            BlackPiece(Bishop),
            BlackPiece(Queen),
            BlackPiece(Pawn),
        );
        #[allow(clippy::deprecated_cfg_attr)]
        Self {
            #[cfg_attr(rustfmt, rustfmt_skip)]
            data: [
                II, II, II, II, II, II, II, II, II, II, II, II, II, II, II, II, II, II, II, II, II,
                WR, WN, WB, WQ, WK, WB, WN, WR,                                             II, II,
                WP, WP, WP, WP, WP, WP, WP, WP,                                             II, II,
                EE, EE, EE, EE, EE, EE, EE, EE,                                             II, II,
                EE, EE, EE, EE, EE, EE, EE, EE,                                             II, II,
                EE, EE, EE, EE, EE, EE, EE, EE,                                             II, II,
                EE, EE, EE, EE, EE, EE, EE, EE,                                             II, II,
                BP, BP, BP, BP, BP, BP, BP, BP,                                             II, II,
                BR, BN, BB, BQ, BK, BB, BN, BR,
                II, II, II, II, II, II, II, II, II, II, II, II, II, II, II, II, II, II, II, II, II,
            ],
            en_passant_chance: Some(Coord::from_xy(3, 3)),
            threat_mask: ThreatMask::new(),
            restore_stack: crate::moves::RestoreStack::new(),
        }
    }

    pub fn get<C: CommonCoord>(&self, coord: C) -> &Field {
        unsafe { self.data.get_unchecked(coord.raw() as usize) }
    }

    pub fn get_if_safe(&self, coord: UnsafeCoord) -> Option<(Coord, Field)> {
        let field = self.get(coord);
        if let Field::Invincible = field {
            None
        } else {
            Some((unsafe { Coord::new_unchecked(coord.0) }, *field))
        }
    }

    pub fn get_mut(&mut self, coord: Coord) -> &mut Field {
        unsafe { self.data.get_unchecked_mut(coord.raw() as usize) }
    }

    pub fn move_piece(&mut self, from: Coord, to: Coord, replace: Field) -> Field {
        let old = *self.get(to);
        *self.get_mut(to) = *self.get(from);
        *self.get_mut(from) = replace;
        old
    }

    pub fn pop_field(&mut self, coord: Coord, mut value: Field) -> Field {
        core::mem::swap(self.get_mut(coord), &mut value);
        value
    }

    pub fn output_terminal(&self, number_hints: bool, highlights: &[Coord]) {
        let sidebar_color = "48;5;194;38;5;0";
        let c1 = || {
            if number_hints {
                print!("\x1b[0;{};1m   ", sidebar_color);
                for x in "ABCDEFGH".chars() {
                    print!("  {}  ", x);
                }
                println!("   \x1b[m")
            }
        };
        let c4 = |b, y| {
            if b {
                print!("\x1b[0;{};1m {} \x1b[m", sidebar_color, y + 1);
            } else {
                print!("\x1b[0;{};1m   \x1b[m", sidebar_color);
            }
        };
        let c2 = |b, x, y, c| {
            if number_hints && x == 0 {
                c4(b, y)
            }
            let coord = Coord::from_xy(x, y);
            let f = |nc, hc| highlights.iter().find(|&v| v == &coord).map_or(nc, |_| hc);
            if (x ^ y) & 1 == 0 {
                print!("\x1b[48;5;{}m  {}  ", f(89, 203), c)
            } else {
                print!("\x1b[48;5;{}m  {}  ", f(36, 203), c)
            }
            if number_hints && x == 7 {
                c4(b, y)
            }
        };
        let c3 = |b, y, f: fn(&Self, i8, i8) -> String| {
            for x in 0..8 {
                c2(b, x, y, f(self, x, y));
            }
            println!()
        };
        c1();
        for y in (0..8).rev() {
            c3(false, y, |_, _, _| " ".to_string());
            c3(true, y, |s: &Self, x, y| {
                s.get(Coord::from_xy(x, y)).unicode()
            });
            c3(false, y, |_, _, _| " ".to_string());
        }
        c1()
    }
}

impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}
