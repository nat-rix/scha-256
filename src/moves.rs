use crate::board::{Board, Color, Coord, Field, Piece};
use crate::list::List;

pub type MoveList = List<Move, 27>;
pub(crate) type RestoreStack = List<RestoreEntry, 256>;

#[derive(Debug, Clone)]
pub enum RestoreType {
    Move(Coord, Coord),
    Capture(Coord, Coord, Field),
    EnPassant(Coord, Coord, Coord, Color),
}

#[derive(Debug, Clone)]
pub struct RestoreEntry {
    pub en_passant_chance: Option<Coord>,
    pub restore_type: RestoreType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
}

impl Direction {
    pub fn as_vector(self) -> i8 {
        match self {
            Self::Left => -1,
            Self::Right => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromotionType {
    Regular,
    Capture,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveType {
    Regular,
    RegularPawnDoubleForward,
    Capture,
    Promote(Piece, PromotionType),
    EnPassant(Coord),
    Castle(Direction),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Move {
    pub start: Coord,
    pub end: Coord,
    pub move_type: MoveType,
}

impl Board {
    pub(crate) fn list_king_moves<const N: usize>(
        &self,
        coord: Coord,
        color: Color,
        into: &mut List<Move, N>,
    ) {
        // TODO: king's moves
        // TODO: castling
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
            Field::BlackKing => self.list_king_moves(coord, Color::Black, into),
            Field::WhiteKing => self.list_king_moves(coord, Color::White, into),
            Field::BlackPiece(piece) => self.list_piece_moves(coord, *piece, Color::Black, into),
            Field::WhitePiece(piece) => self.list_piece_moves(coord, *piece, Color::White, into),
        }
    }

    pub fn enumerate_moves(&self, coord: Coord) -> MoveList {
        let mut list = MoveList::new();
        self.add_moves(coord, &mut list);
        list
    }

    pub fn do_move(&mut self, mv: Move) {
        self.remove_threat_mask_piece_moves(mv);
        let old_en_passant_chance = self.en_passant_chance;
        self.en_passant_chance = None;
        match mv.move_type {
            MoveType::Regular => {
                self.move_piece(mv.start, mv.end, Field::Empty);
                self.restore_stack.append(RestoreEntry {
                    en_passant_chance: old_en_passant_chance,
                    restore_type: RestoreType::Move(mv.end, mv.start),
                })
            }
            MoveType::RegularPawnDoubleForward => {
                self.move_piece(mv.start, mv.end, Field::Empty);
                self.restore_stack.append(RestoreEntry {
                    en_passant_chance: old_en_passant_chance,
                    restore_type: RestoreType::Move(mv.end, mv.start),
                });
                self.en_passant_chance = Some(mv.end)
            }
            MoveType::Capture => {
                let old = self.move_piece(mv.start, mv.end, Field::Empty);
                self.restore_stack.append(RestoreEntry {
                    en_passant_chance: old_en_passant_chance,
                    restore_type: RestoreType::Capture(mv.end, mv.start, old),
                })
            }
            MoveType::Promote(piece, _promotion_type) => {
                let new_field = match self.pop_field(mv.start, Field::Empty) {
                    Field::BlackPiece(_) => Field::BlackPiece(piece),
                    Field::WhitePiece(_) => Field::WhitePiece(piece),
                    v => v,
                };
                let restore_type = match self.pop_field(mv.end, new_field) {
                    Field::Empty => RestoreType::Move(mv.end, mv.start),
                    old => RestoreType::Capture(mv.end, mv.start, old),
                };
                self.restore_stack.append(RestoreEntry {
                    en_passant_chance: old_en_passant_chance,
                    restore_type,
                })
            }
            MoveType::EnPassant(target) => {
                self.move_piece(mv.start, mv.end, Field::Empty);
                let old = self.pop_field(target, Field::Empty);
                self.restore_stack.append(RestoreEntry {
                    en_passant_chance: old_en_passant_chance,
                    restore_type: RestoreType::EnPassant(
                        mv.end,
                        mv.start,
                        target,
                        match old {
                            Field::WhitePiece(_) => Color::White,
                            _ => Color::Black,
                        },
                    ),
                })
            }
            MoveType::Castle(_dir) => {
                // TODO: castling
            }
        }
        self.update_threat_mask_with(mv)
    }
}
