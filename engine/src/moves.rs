use crate::board::{Board, Color, Coord, Field, Piece};
use crate::list::List;
use crate::threat::{Direction, King};

const MAX_MOVES: usize = 27;
const MAX_PIECES: usize = 16;
pub type MoveList = List<Move, MAX_MOVES>;
pub type LongMoveList = List<Move, { MAX_PIECES * MAX_MOVES }>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromotionType {
    Regular,
    Capture,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Castle {
    pub rook_pos: Coord,
    pub rook_target: Coord,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveType {
    Regular,
    RegularPawnDoubleForward,
    Capture,
    Promote(Piece, PromotionType),
    EnPassant(Coord),
    Castle(Castle),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Move {
    pub start: Coord,
    pub end: Coord,
    pub move_type: MoveType,
}

impl Board {
    fn is_bad_king_move(&self, target: Coord, color: Color) -> bool {
        for &threat in self.get_king(color).aggressors.slice() {
            let f = self.get(threat);
            if target == threat {
                continue;
            }
            let (x1, y1) = target.as_xy();
            let (x2, y2) = threat.as_xy();
            let diag = || (x1 - x2).abs() == (y1 - y2).abs();
            let hor = || x1 == x2 || y1 == y2;
            if match f {
                Field::BlackPiece(Piece::Bishop) | Field::WhitePiece(Piece::Bishop) => diag(),
                Field::BlackPiece(Piece::Rook) | Field::WhitePiece(Piece::Rook) => hor(),
                Field::BlackPiece(Piece::Queen) | Field::WhitePiece(Piece::Queen) => {
                    diag() || hor()
                }
                _ => false,
            } {
                return true;
            }
        }
        false
    }

    pub(crate) fn list_king_moves<const N: usize>(
        &self,
        coord: Coord,
        color: Color,
        is_in_check: bool,
        into: &mut List<Move, N>,
    ) {
        for &(dx, dy) in &[
            (1, 0),
            (-1, 0),
            (0, 1),
            (0, -1),
            (-1, 1),
            (-1, -1),
            (1, 1),
            (1, -1),
        ] {
            if let Some((target_coord, field)) = self.get_if_safe(coord.rel(dx, dy)) {
                if !self.get_threatened_by(target_coord, color) {
                    if matches!(field, Field::Empty) {
                        if !is_in_check || !self.is_bad_king_move(target_coord, color) {
                            into.append(Move {
                                start: coord,
                                end: target_coord,
                                move_type: MoveType::Regular,
                            })
                        }
                    } else if field.is_color_piece(color)
                        && (!is_in_check || !self.is_bad_king_move(target_coord, color))
                    {
                        into.append(Move {
                            start: coord,
                            end: target_coord,
                            move_type: MoveType::Capture,
                        })
                    }
                }
            }
        }
        if !is_in_check {
            let king = self.get_king(color);
            let mut castle_d = |d1, d2, d3| {
                if let (Some((rook_target, Field::Empty)), Some((king_target, Field::Empty))) = (
                    self.get_if_safe(coord.rel(d1, 0)),
                    self.get_if_safe(coord.rel(d2, 0)),
                ) {
                    if let (Color::Black, Some((rook_pos, Field::BlackPiece(Piece::Rook))))
                    | (Color::White, Some((rook_pos, Field::WhitePiece(Piece::Rook)))) =
                        (color, self.get_if_safe(coord.rel(d3, 0)))
                    {
                        if !self.get_threatened_by(rook_target, color)
                            && !self.get_threatened_by(king_target, color)
                        {
                            into.append(Move {
                                start: coord,
                                end: king_target,
                                move_type: MoveType::Castle(Castle {
                                    rook_pos,
                                    rook_target,
                                }),
                            })
                        }
                    }
                }
            };
            if king.castling_to_right {
                castle_d(1, 2, 3);
            }
            if king.castling_to_left {
                castle_d(-1, -2, -4);
            }
        }
    }

    pub(crate) fn list_pawn_moves<const N: usize>(
        &self,
        coord: Coord,
        color: Color,
        into: &mut List<Move, N>,
    ) {
        let delta = if let Color::White = color { 1 } else { -1 };
        let (forward1_coord, forward2_coord) = (coord.rel(0, delta), coord.rel(0, delta * 2));
        let endline_reaching = forward1_coord.endline() == Some(color);
        let append_mut_if_endline = |into: &mut List<Move, N>, start, end, promotion_type| {
            if endline_reaching {
                for &piece in &[
                    Piece::Queen,
                    Piece::Rook,
                    Piece::Knight,
                    Piece::Bishop,
                    Piece::Pawn,
                ] {
                    into.append(Move {
                        start,
                        end,
                        move_type: MoveType::Promote(piece, promotion_type),
                    })
                }
            } else {
                into.append(Move {
                    start,
                    end,
                    move_type: match promotion_type {
                        PromotionType::Regular => MoveType::Regular,
                        PromotionType::Capture => MoveType::Capture,
                    },
                })
            }
        };
        if let Some((forward1_coord, Field::Empty)) = self.get_if_safe(forward1_coord) {
            append_mut_if_endline(into, coord, forward1_coord, PromotionType::Regular);
            if coord.baseline() == Some(color) {
                if let Some((forward2_coord, Field::Empty)) = self.get_if_safe(forward2_coord) {
                    into.append(Move {
                        start: coord,
                        end: forward2_coord,
                        move_type: MoveType::RegularPawnDoubleForward,
                    });
                }
            }
        }
        for &target_coord in &[coord.rel(-1, delta), coord.rel(1, delta)] {
            if let Some((target_coord, field)) = self.get_if_safe(target_coord) {
                if field.is_color_piece(color) {
                    append_mut_if_endline(into, coord, target_coord, PromotionType::Capture)
                }
            }
        }
        if let Some(target_coord) = self.en_passant_chance {
            if let Some((jump_coord, _)) = self.get_if_safe(target_coord.rel(0, delta)) {
                if target_coord.as_unsafe() == coord.rel(1, 0)
                    || target_coord.as_unsafe() == coord.rel(-1, 0)
                {
                    into.append(Move {
                        start: coord,
                        end: jump_coord,
                        move_type: MoveType::EnPassant(target_coord),
                    })
                }
            }
        }
    }

    pub(crate) fn list_directional_moves<const N: usize>(
        &self,
        coord: Coord,
        color: Color,
        directions: &[(i8, i8)],
        into: &mut List<Move, N>,
    ) {
        for &(dx, dy) in directions {
            let mut target_coord = coord;
            loop {
                let target_coord_unsafe = target_coord.rel(dx, dy);
                if let Some((target_coord_safe, field)) = self.get_if_safe(target_coord_unsafe) {
                    if matches!(field, Field::Empty) {
                        into.append(Move {
                            start: coord,
                            end: target_coord_safe,
                            move_type: MoveType::Regular,
                        })
                    } else if field.is_color_piece(color) {
                        into.append(Move {
                            start: coord,
                            end: target_coord_safe,
                            move_type: MoveType::Capture,
                        });
                        break;
                    } else {
                        break;
                    }
                    target_coord = target_coord_safe;
                } else {
                    break;
                }
            }
        }
    }

    pub(crate) fn list_rook_moves<const N: usize>(
        &self,
        coord: Coord,
        color: Color,
        into: &mut List<Move, N>,
    ) {
        self.list_directional_moves(coord, color, &[(1, 0), (-1, 0), (0, 1), (0, -1)], into)
    }

    pub(crate) fn list_bishop_moves<const N: usize>(
        &self,
        coord: Coord,
        color: Color,
        into: &mut List<Move, N>,
    ) {
        self.list_directional_moves(coord, color, &[(1, 1), (-1, 1), (1, -1), (-1, -1)], into)
    }

    pub(crate) fn list_knight_moves<const N: usize>(
        &self,
        coord: Coord,
        color: Color,
        into: &mut List<Move, N>,
    ) {
        for &(dx, dy) in &[
            (2, 1),
            (2, -1),
            (1, 2),
            (1, -2),
            (-2, 1),
            (-2, -1),
            (-1, 2),
            (-1, -2),
        ] {
            if let Some((target_coord, field)) = self.get_if_safe(coord.rel(dx, dy)) {
                if matches!(field, Field::Empty) {
                    into.append(Move {
                        start: coord,
                        end: target_coord,
                        move_type: MoveType::Regular,
                    })
                } else if field.is_color_piece(color) {
                    into.append(Move {
                        start: coord,
                        end: target_coord,
                        move_type: MoveType::Capture,
                    })
                }
            }
        }
    }

    pub(crate) fn list_queen_moves<const N: usize>(
        &self,
        coord: Coord,
        color: Color,
        into: &mut List<Move, N>,
    ) {
        self.list_directional_moves(
            coord,
            color,
            &[
                (1, 0),
                (-1, 0),
                (0, 1),
                (0, -1),
                (1, 1),
                (-1, 1),
                (1, -1),
                (-1, -1),
            ],
            into,
        )
    }

    pub(crate) fn list_piece_moves<const N: usize>(
        &self,
        coord: Coord,
        piece: Piece,
        color: Color,
        into: &mut List<Move, N>,
    ) {
        (match piece {
            Piece::Pawn => Self::list_pawn_moves,
            Piece::Rook => Self::list_rook_moves,
            Piece::Bishop => Self::list_bishop_moves,
            Piece::Knight => Self::list_knight_moves,
            Piece::Queen => Self::list_queen_moves,
        })(self, coord, color, into)
    }

    fn add_moves<const N: usize>(&self, coord: Coord, into: &mut List<Move, N>) {
        match self.get(coord) {
            Field::Empty | Field::Invincible => (),
            Field::BlackKing => self.list_king_moves(coord, Color::Black, false, into),
            Field::WhiteKing => self.list_king_moves(coord, Color::White, false, into),
            Field::BlackPiece(piece) => self.list_piece_moves(coord, *piece, Color::Black, into),
            Field::WhitePiece(piece) => self.list_piece_moves(coord, *piece, Color::White, into),
        }
    }

    fn add_moves_check<const N: usize>(&self, coord: Coord, into: &mut List<Move, N>) {
        let n = into.slice().len();
        match self.get(coord) {
            Field::Empty | Field::Invincible => (),
            Field::BlackKing => self.list_king_moves(coord, Color::Black, true, into),
            Field::WhiteKing => self.list_king_moves(coord, Color::White, true, into),
            Field::BlackPiece(piece) => {
                self.list_piece_moves(coord, *piece, Color::Black, into);
                self.filter_checks(Color::Black, n, into)
            }
            Field::WhitePiece(piece) => {
                self.list_piece_moves(coord, *piece, Color::White, into);
                self.filter_checks(Color::White, n, into)
            }
        }
    }

    pub fn is_in_check(&self, color: Color) -> bool {
        self.get_threatened_by(self.get_king(color).coord, color)
    }

    pub fn enumerate_moves(&self, color: Color, coord: Coord) -> MoveList {
        let mut list = MoveList::new();
        let king = self.get_king(color);
        if king.aggressors.is_empty() {
            self.add_moves(coord, &mut list)
        } else {
            self.add_moves_check(coord, &mut list)
        }
        self.filter_potential_checks(king, 0, &mut list);
        list
    }

    pub fn enumerate_all_moves_by(&self, color: Color, list: &mut LongMoveList) {
        let king = self.get_king(color);
        let f = if king.aggressors.is_empty() {
            Self::add_moves
        } else {
            Self::add_moves_check
        };
        let mut n = 21;
        for _ in 0..8 {
            for _ in 0..8 {
                let coord = Coord(unsafe { core::num::NonZeroI8::new_unchecked(n) });
                if self.get(coord).is_color_piece_include_king(!color) {
                    let nbefore = list.slice().len();
                    f(self, coord, list);
                    self.filter_potential_checks(king, nbefore, list);
                }
                n += 1;
            }
            n += 2;
        }
    }

    pub fn is_potential_check(&self, king: &King, mv: &Move) -> bool {
        let pc = king.get_potential_check(mv.start);
        if let Some((coord, d)) = pc {
            if &mv.end == coord {
                return false;
            }
            if let Field::BlackPiece(piece) | Field::WhitePiece(piece) = self.get(mv.start) {
                if let Piece::Knight = piece {
                    true
                } else {
                    let (start, end) = (mv.start.0.get(), mv.end.0.get());
                    let (sx, sy) = (start % 10, start / 10);
                    let (ex, ey) = (end % 10, end / 10);
                    use core::cmp::Ordering::*;
                    match (sx.cmp(&ex), sy.cmp(&ey)) {
                        (Equal, _) => d != &Direction::Up && d != &Direction::Down,
                        (_, Equal) => d != &Direction::Left && d != &Direction::Right,
                        (Greater, Greater) | (Less, Less) => {
                            d != &Direction::UpRight && d != &Direction::DownLeft
                        }
                        (Greater, Less) | (Less, Greater) => {
                            d != &Direction::UpLeft && d != &Direction::DownRight
                        }
                    }
                }
            } else {
                false
            }
        } else {
            false
        }
    }

    fn is_check_saving_piece(&self, threat: Coord, piece: Piece, king: &King, mv: &Move) -> bool {
        if mv.end == threat {
            return true;
        }
        if let Piece::Pawn | Piece::Knight = piece {
            return false;
        }
        if !self.threat_mask.get(mv.end).slice().contains(&threat) {
            return false;
        }
        let etok = king.coord.0.get() - mv.end.0.get();
        let ttoe = mv.end.0.get() - threat.0.get();
        if etok.is_positive() != ttoe.is_positive() {
            return false;
        }
        let (a, b) = (etok.abs(), ttoe.abs());
        let mf = |n| a.abs() % n == 0 && b.abs() % n == 0;
        let eq = |x, y, z| x == y && y == z;
        etok == ttoe
            || mf(9)
            || mf(10)
            || mf(11)
            || eq(
                king.coord.0.get() / 10,
                mv.end.0.get() / 10,
                threat.0.get() / 10,
            )
    }

    pub fn is_check_saving(&self, color: Color, mv: &Move) -> bool {
        let king = self.get_king(color);
        if let [threat] = king.aggressors.slice() {
            let f = self.get(*threat);
            match f {
                Field::BlackPiece(p) | Field::WhitePiece(p) => {
                    self.is_check_saving_piece(*threat, *p, king, mv)
                }
                _ => false,
            }
        } else {
            false
        }
    }

    pub fn filter_checks<const N: usize>(
        &self,
        color: Color,
        start: usize,
        list: &mut List<Move, N>,
    ) {
        list.filter(start, |v| self.is_check_saving(color, v))
    }

    pub fn filter_potential_checks<const N: usize>(
        &self,
        king: &King,
        start: usize,
        list: &mut List<Move, N>,
    ) {
        list.filter(start, |v| !self.is_potential_check(king, v))
    }

    pub fn do_move(&mut self, mv: Move) {
        self.remove_threat_mask_piece_moves(mv);
        match self.get(mv.start) {
            Field::BlackKing => {
                self.black_king.coord = mv.end;
                self.black_king.castling_to_left = false;
                self.black_king.castling_to_right = false;
            }
            Field::WhiteKing => {
                self.white_king.coord = mv.end;
                self.white_king.castling_to_left = false;
                self.white_king.castling_to_right = false;
            }
            Field::WhitePiece(Piece::Rook) => {
                if mv.start == Coord::from_xy(0, 0) {
                    self.white_king.castling_to_left = false;
                } else {
                    self.white_king.castling_to_right = false;
                }
            }
            Field::BlackPiece(Piece::Rook) => {
                if mv.start == Coord::from_xy(0, 7) {
                    self.black_king.castling_to_left = false;
                } else {
                    self.black_king.castling_to_right = false;
                }
            }
            _ => (),
        };
        self.en_passant_chance = None;
        match mv.move_type {
            MoveType::Regular => {
                self.move_piece(mv.start, mv.end, Field::Empty);
            }
            MoveType::RegularPawnDoubleForward => {
                self.move_piece(mv.start, mv.end, Field::Empty);
                self.en_passant_chance = Some(mv.end)
            }
            MoveType::Capture => {
                self.move_piece(mv.start, mv.end, Field::Empty);
            }
            MoveType::Promote(piece, _promotion_type) => {
                let new_field = match self.pop_field(mv.start, Field::Empty) {
                    Field::BlackPiece(_) => Field::BlackPiece(piece),
                    Field::WhitePiece(_) => Field::WhitePiece(piece),
                    v => v,
                };
                self.pop_field(mv.end, new_field);
            }
            MoveType::EnPassant(target) => {
                self.move_piece(mv.start, mv.end, Field::Empty);
                self.pop_field(target, Field::Empty);
            }
            MoveType::Castle(Castle {
                rook_pos,
                rook_target,
            }) => {
                self.move_piece(mv.start, mv.end, Field::Empty);
                self.move_piece(rook_pos, rook_target, Field::Empty);
            }
        }
        self.update_threat_mask_with(mv);
        self.update_potential_checks();
    }
}
