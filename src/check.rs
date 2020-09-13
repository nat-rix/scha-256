use crate::board::{Board, Color, CommonCoord, Coord, Field, Piece};
use crate::list::{array_from_fn, List};
use crate::moves::Move;

#[derive(Debug, Clone)]
pub struct Checks;

impl Checks {
    pub const fn empty() -> Self {
        Self
    }
}

#[derive(Debug, Clone)]
pub struct ThreatMask {
    threats: [List<Coord, 16>; 10 * 12],
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
        let mut threats: [List<Coord, 16>; 10 * 12] = array_from_fn(List::new);
        let mut append = |i, v| threats[i as usize].append(unsafe { Coord::new_unchecked(v) });
        for (b, f) in &[(40i8, sub), (70i8, add)] {
            for &(i, v) in &[(1, 2), (8, 7)] {
                append(b + i, f(*b, 10) + v); // border pawns
                append(f(b + i, 10), f(b + i, 20)); // rook -> pawns
                append(f(b + v, 20), f(b + i, 20)); // rook -> knights
            }
            for i in (b + 2)..(b + 8) {
                append(i, f(i, 9));
                append(i, f(i, 11));
            }
        }
        Self { threats }
    }

    pub fn get<C: CommonCoord>(&self, coord: C) -> &[Coord] {
        unsafe { self.threats.get_unchecked(coord.raw() as usize) }.slice()
    }

    pub fn get_mut<C: CommonCoord>(&mut self, coord: C) -> &mut List<Coord, 16> {
        unsafe { self.threats.get_unchecked_mut(coord.raw() as usize) }
    }
}

impl Board {
    pub fn update_threat_mask_with(mv: Move) {}
}
