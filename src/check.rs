use crate::board::{Board, CommonCoord, Coord, Field, Piece};
use crate::list::{array_from_fn, List};
use crate::moves::{Move, MoveType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreatVector {
    Left,
    Right,
    Top,
    Bottom,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Debug, Clone)]
pub struct ThreatMask {
    threats: [List<(Coord, Option<ThreatVector>), 16>; 10 * 12],
}

impl Default for ThreatMask {
    fn default() -> Self {
        Self::new()
    }
}

impl ThreatMask {
    pub fn new() -> Self {
        let [add, sub]: [&dyn Fn(i8, i8) -> i8; 2] =
            [&<i8 as core::ops::Add>::add, &<i8 as core::ops::Sub>::sub];
        let mut threats: [List<(Coord, Option<ThreatVector>), 16>; 10 * 12] =
            array_from_fn(List::new);
        let mut append =
            |i, v, d| threats[i as usize].append((unsafe { Coord::new_unchecked(v) }, d));
        for (b, f) in &[(40i8, sub), (70i8, add)] {
            for &(i, i2, i3, i4, i5) in &[(1, 2, 3, 4, 5), (8, 7, 6, 5, 4)] {
                append(b + i, f(b + i2, 10), None); // border pawns -> empty field
                append(b + i, f(b + i2, 20), None); // knight -> empty field border
                append(b + i3, f(b + i2, 20), None); // knight -> empty field
                append(f(b + i, 10), f(b + i, 20), Some()); // rook -> pawns
                append(f(b + i2, 20), f(b + i, 20), None); // rook -> knights
                append(f(b + i2, 10), f(b + i3, 20), Some()); // bishop -> border pawn
                append(f(b + i4, 10), f(b + i3, 20), Some()); // bishop -> border pawn
                append(f(b + i4, 10), f(b + i2, 20), None); // knight -> pawn
                append(f(b + i4, 10), f(b + 4, 20), Some()); // queen -> center pawns
                append(f(b + i4, 10), f(b + 5, 20), None); // king -> center pawns
                append(f(b + i3, 20), f(b + i4, 20), Some()); // queen/king -> bishop
                append(f(b + i5, 20), f(b + i4, 20)); // queen/king -> queen/king
            }
            append(f(b + 3, 10), f(b + 4, 20)); // queen -> left pawn
            append(f(b + 6, 10), f(b + 5, 20)); // king -> right pawn
            for i in (b + 2)..(b + 8) {
                append(i, f(i, 9));
                append(i, f(i, 11));
            }
        }
        Self { threats }
    }

    pub fn get<C: CommonCoord>(&self, coord: C) -> &List<Coord, 16> {
        unsafe { self.threats.get_unchecked(coord.raw() as usize) }
    }

    pub fn get_mut<C: CommonCoord>(&mut self, coord: C) -> &mut List<Coord, 16> {
        unsafe { self.threats.get_unchecked_mut(coord.raw() as usize) }
    }
}

impl Board {
    fn update_threat_mask_remove_barrier_rook(&mut self, threat: Coord, target: Coord) {
        let (dx, dy) = match threat.0.get() - target.0.get() {
            1..=8 => (1, 0),
            9..=core::i8::MAX => (0, 1),
            -8..=0 => (-1, 0),
            core::i8::MIN..=-9 => (0, -1),
        };
    }

    fn update_threat_mask_remove_barrier_bishop(&mut self, threat: Coord, target: Coord) {}

    fn update_threat_mask_remove_barrier(&mut self, coord: Coord) {
        for &threat in self.threat_mask.get(coord).clone().slice() {
            let field = self.get(threat);
            match field {
                Field::BlackPiece(Piece::Queen) | Field::WhitePiece(Piece::Queen) => {
                    self.update_threat_mask_remove_barrier_rook(threat, coord);
                    self.update_threat_mask_remove_barrier_bishop(threat, coord);
                }
                Field::BlackPiece(Piece::Rook) | Field::WhitePiece(Piece::Rook) => {
                    self.update_threat_mask_remove_barrier_rook(threat, coord)
                }
                Field::BlackPiece(Piece::Bishop) | Field::WhitePiece(Piece::Bishop) => {
                    self.update_threat_mask_remove_barrier_bishop(threat, coord)
                }
                _ => (),
            }
        }
    }

    fn update_threat_mask_add_barrier(&mut self, coord: Coord) {}
    fn update_threat_mask_add_piece(&mut self, coord: Coord) {}

    pub(crate) fn update_threat_mask_with(&mut self, mv: Move) {
        match mv.move_type {
            MoveType::Regular
            | MoveType::RegularPawnDoubleForward
            | MoveType::Capture
            | MoveType::Promote(_, _) => {
                self.update_threat_mask_remove_barrier(mv.start);
                self.update_threat_mask_add_barrier(mv.end);
                self.update_threat_mask_add_piece(mv.end);
            }
            MoveType::EnPassant(target) => {
                self.update_threat_mask_remove_barrier(mv.start);
                self.update_threat_mask_remove_barrier(target);
                self.update_threat_mask_add_barrier(mv.end);
                self.update_threat_mask_add_piece(mv.end);
            }
            MoveType::Castle(_dir) => {
                // TODO: castling
            }
        }
    }

    pub(crate) fn remove_threat_mask_piece_moves(&mut self, coord: Coord) {
        // TOOD
    }
}
