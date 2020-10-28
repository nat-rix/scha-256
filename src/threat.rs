use crate::board::{Board, Color, CommonCoord, Coord, Field, Piece, UnsafeCoord};
use crate::list::{array_from_fn, List};
use crate::moves::{Move, MoveType, PromotionType};

#[derive(Debug, Clone)]
pub struct King {
    pub coord: Coord,
    pub potential_check_map: [Option<(Coord, Direction)>; 10 * 12],
    pub castling_to_left: bool,
    pub castling_to_right: bool,
}

impl King {
    pub const fn new(color: Color) -> Self {
        Self {
            coord: if let Color::Black = color {
                Coord::from_xy(4, 7)
            } else {
                Coord::from_xy(4, 0)
            },
            potential_check_map: [None; 10 * 12],
            castling_to_left: true,
            castling_to_right: true,
        }
    }

    pub fn get_potential_check<C: CommonCoord>(&self, coord: C) -> &Option<(Coord, Direction)> {
        unsafe { self.potential_check_map.get_unchecked(coord.raw() as usize) }
    }

    pub fn get_potential_check_mut<C: CommonCoord>(
        &mut self,
        coord: C,
    ) -> &mut Option<(Coord, Direction)> {
        unsafe {
            self.potential_check_map
                .get_unchecked_mut(coord.raw() as usize)
        }
    }

    pub fn clean_check_map(&mut self) {
        for i in &mut self.potential_check_map {
            *i = None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
    UpLeft,
    UpRight,
    DownLeft,
    DownRight,
}

impl Direction {
    pub const fn get_xy(&self) -> (i8, i8) {
        match self {
            Self::Up => (0, 1),
            Self::Down => (0, -1),
            Self::Left => (-1, 0),
            Self::Right => (1, 0),
            Self::UpLeft => (-1, 1),
            Self::UpRight => (1, 1),
            Self::DownLeft => (-1, -1),
            Self::DownRight => (1, -1),
        }
    }

    pub const fn is_diagonal(&self) -> bool {
        match self {
            Self::Up | Self::Down | Self::Left | Self::Right => false,
            Self::UpRight | Self::DownRight | Self::UpLeft | Self::DownLeft => true,
        }
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
        Self {
            threats: array_from_fn(List::new),
        }
    }

    pub fn get<C: CommonCoord>(&self, coord: C) -> &List<Coord, 16> {
        unsafe { self.threats.get_unchecked(coord.raw() as usize) }
    }

    pub fn get_mut<C: CommonCoord>(&mut self, coord: C) -> &mut List<Coord, 16> {
        unsafe { self.threats.get_unchecked_mut(coord.raw() as usize) }
    }
}

impl Board {
    fn directional_repeat<F: FnMut(&mut Self, Coord)>(
        &mut self,
        mut iter: Coord,
        (dx, dy): (i8, i8),
        mut f: F,
    ) {
        loop {
            let next = iter.rel(dx, dy);
            let field = self.get(next);
            match field {
                Field::Empty => f(self, unsafe { next.as_safe_unchecked() }),
                Field::BlackPiece(_)
                | Field::WhitePiece(_)
                | Field::BlackKing
                | Field::WhiteKing => {
                    f(self, unsafe { next.as_safe_unchecked() });
                    break;
                }
                _ => break,
            }
            iter = unsafe { next.as_safe_unchecked() };
        }
    }

    fn update_threat_mask_modify_barrier_rook<F: FnMut(&mut Self, Coord, Coord)>(
        &mut self,
        threat: Coord,
        target: Coord,
        f: &mut F,
    ) {
        let (dx, dy) = match threat.0.get() - target.0.get() {
            1..=8 => (-1, 0),
            9..=core::i8::MAX => (0, -1),
            -8..=0 => (1, 0),
            core::i8::MIN..=-9 => (0, 1),
        };
        self.directional_repeat(target, (dx, dy), |s, n| f(s, n, threat));
    }

    fn update_threat_mask_modify_barrier_bishop<F: FnMut(&mut Self, Coord, Coord)>(
        &mut self,
        threat: Coord,
        target: Coord,
        f: &mut F,
    ) {
        let dif = threat.0.get() - target.0.get();
        let modulo = dif % 9;
        let (dx, dy) = match (dif.is_negative(), modulo) {
            (true, 0) => (-1, 1),
            (true, _) => (1, 1),
            (false, 0) => (1, -1),
            (false, _) => (-1, -1),
        };
        self.directional_repeat(target, (dx, dy), |s, n| f(s, n, threat));
    }

    fn update_threat_mask_modify_barrier_queen<F: FnMut(&mut Self, Coord, Coord)>(
        &mut self,
        threat: Coord,
        target: Coord,
        f: &mut F,
    ) {
        let dif = threat.0.get() - target.0.get();
        let modulo9 = dif % 9;
        let modulo11 = dif % 11;
        let (dx, dy) = match (dif, dif.is_negative(), modulo9, modulo11) {
            (_, true, 0, _) => (-1, 1),
            (_, true, _, 0) => (1, 1),
            (_, false, 0, _) => (1, -1),
            (_, false, _, 0) => (-1, -1),
            (1..=8, _, _, _) => (-1, 0),
            (9..=core::i8::MAX, _, _, _) => (0, -1),
            (-8..=0, _, _, _) => (1, 0),
            (core::i8::MIN..=-9, _, _, _) => (0, 1),
        };
        self.directional_repeat(target, (dx, dy), |s, n| f(s, n, threat));
    }

    fn update_threat_mask_modify_barrier<F: FnMut(&mut Self, Coord, Coord)>(
        &mut self,
        coord: Coord,
        f: &mut F,
    ) {
        for &threat in self.threat_mask.get(coord).clone().slice() {
            let field = self.get(threat);
            match field {
                Field::BlackPiece(Piece::Queen) | Field::WhitePiece(Piece::Queen) => {
                    self.update_threat_mask_modify_barrier_queen(threat, coord, f)
                }
                Field::BlackPiece(Piece::Rook) | Field::WhitePiece(Piece::Rook) => {
                    self.update_threat_mask_modify_barrier_rook(threat, coord, f)
                }
                Field::BlackPiece(Piece::Bishop) | Field::WhitePiece(Piece::Bishop) => {
                    self.update_threat_mask_modify_barrier_bishop(threat, coord, f)
                }
                _ => (),
            }
        }
    }

    fn update_threat_mask_add_barrier(&mut self, coord: Coord) {
        let mut f =
            |s: &mut Self, target, threat| s.threat_mask.get_mut(target).swap_remove(&threat);
        self.update_threat_mask_modify_barrier(coord, &mut f)
    }

    fn update_threat_mask_remove_barrier(&mut self, coord: Coord) {
        let mut f = |s: &mut Self, target, threat| s.threat_mask.get_mut(target).append(threat);
        self.update_threat_mask_modify_barrier(coord, &mut f)
    }

    fn update_threat_mask_add_piece(&mut self, coord: Coord) {
        for &target in self.get_causing_threats(coord).slice() {
            if let Some((target, _)) = self.get_if_safe(target) {
                self.threat_mask.get_mut(target).append(coord)
            }
        }
    }

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

    pub(crate) fn init_threat_mask(&mut self) {
        for &y in &[0, 1, 6, 7] {
            for x in 0..8 {
                let threat = Coord::from_xy(x, y);
                for &target in self.get_causing_threats(threat).slice() {
                    if let Some((target, _)) = self.get_if_safe(target) {
                        self.threat_mask.get_mut(target).append(threat)
                    }
                }
            }
        }
    }

    fn get_causing_king_threats<const N: usize>(
        &mut self,
        list: &mut List<UnsafeCoord, N>,
        coord: Coord,
    ) {
        for &(x, y) in &[
            (1, 1),
            (-1, -1),
            (1, -1),
            (-1, 1),
            (1, 0),
            (-1, 0),
            (0, -1),
            (0, 1),
        ] {
            list.append(coord.rel(x, y))
        }
    }

    fn get_causing_pawn_threats<const N: usize>(
        &mut self,
        list: &mut List<UnsafeCoord, N>,
        coord: Coord,
        color: Color,
    ) {
        let d = match color {
            Color::White => 1,
            Color::Black => -1,
        };
        list.append(coord.rel(-1, d));
        list.append(coord.rel(1, d));
    }

    fn get_causing_knight_threats<const N: usize>(
        &mut self,
        list: &mut List<UnsafeCoord, N>,
        coord: Coord,
        _color: Color,
    ) {
        for &(x, y) in &[
            (2, 1),
            (2, -1),
            (1, 2),
            (1, -2),
            (-2, 1),
            (-2, -1),
            (-1, 2),
            (-1, -2),
        ] {
            list.append(coord.rel(x, y))
        }
    }

    fn get_causing_directional_threats<const N: usize>(
        &mut self,
        list: &mut List<UnsafeCoord, N>,
        coord: Coord,
        dx: i8,
        dy: i8,
    ) {
        let mut i = coord;
        loop {
            let next = i.rel(dx, dy);
            list.append(next);
            match self.get(next) {
                Field::Empty => i = unsafe { next.as_safe_unchecked() },
                _ => return,
            }
        }
    }

    fn get_causing_bishop_threats<const N: usize>(
        &mut self,
        list: &mut List<UnsafeCoord, N>,
        coord: Coord,
        _color: Color,
    ) {
        for &(dx, dy) in &[(1, 1), (-1, -1), (1, -1), (-1, 1)] {
            self.get_causing_directional_threats(list, coord, dx, dy)
        }
    }

    fn get_causing_rook_threats<const N: usize>(
        &mut self,
        list: &mut List<UnsafeCoord, N>,
        coord: Coord,
        _color: Color,
    ) {
        for &(dx, dy) in &[(1, 0), (-1, 0), (0, -1), (0, 1)] {
            self.get_causing_directional_threats(list, coord, dx, dy)
        }
    }

    fn get_causing_queen_threats<const N: usize>(
        &mut self,
        list: &mut List<UnsafeCoord, N>,
        coord: Coord,
        _color: Color,
    ) {
        self.get_causing_rook_threats(list, coord, _color);
        self.get_causing_bishop_threats(list, coord, _color);
    }

    fn get_causing_piece_threats<const N: usize>(
        &mut self,
        list: &mut List<UnsafeCoord, N>,
        coord: Coord,
        piece: Piece,
        color: Color,
    ) {
        (match piece {
            Piece::Pawn => Self::get_causing_pawn_threats,
            Piece::Knight => Self::get_causing_knight_threats,
            Piece::Bishop => Self::get_causing_bishop_threats,
            Piece::Rook => Self::get_causing_rook_threats,
            Piece::Queen => Self::get_causing_queen_threats,
        })(self, list, coord, color)
    }

    pub fn get_causing_threats(&mut self, coord: Coord) -> List<UnsafeCoord, 27> {
        let mut list = List::new();
        match self.get(coord) {
            &Field::BlackPiece(piece) => {
                self.get_causing_piece_threats(&mut list, coord, piece, Color::Black)
            }
            &Field::WhitePiece(piece) => {
                self.get_causing_piece_threats(&mut list, coord, piece, Color::White)
            }
            Field::BlackKing | Field::WhiteKing => self.get_causing_king_threats(&mut list, coord),
            _ => (),
        }
        list
    }

    fn remove_threat_mask_piece_at(&mut self, coord: Coord) {
        for &target in self.get_causing_threats(coord).slice() {
            self.threat_mask.get_mut(target).swap_remove(&coord)
        }
    }

    pub(crate) fn remove_threat_mask_piece_moves(&mut self, mv: Move) {
        self.remove_threat_mask_piece_at(mv.start);
        match mv.move_type {
            MoveType::Regular
            | MoveType::RegularPawnDoubleForward
            | MoveType::Promote(_, PromotionType::Regular) => (),
            MoveType::EnPassant(coord) => self.remove_threat_mask_piece_at(coord),
            MoveType::Capture | MoveType::Promote(_, PromotionType::Capture) => {
                self.remove_threat_mask_piece_at(mv.end)
            }
            MoveType::Castle(_dir) => (), // TODO: castling
        }
    }

    fn get_potential_checks(&mut self, coord: Coord, color: Color) -> List<(Coord, Direction), 8> {
        let mut pcs = List::new();
        for dir in &[
            Direction::Up,
            Direction::Down,
            Direction::Left,
            Direction::Right,
            Direction::UpLeft,
            Direction::UpRight,
            Direction::DownLeft,
            Direction::DownRight,
        ] {
            let (dx, dy) = dir.get_xy();
            let mut pos = coord;
            let mut visited = None;
            while let Some((safepos, field)) = self.get_if_safe(pos.rel(dx, dy)) {
                pos = safepos;
                match (visited, color, field) {
                    (None, Color::Black, Field::BlackPiece(_))
                    | (None, Color::Black, Field::BlackKing)
                    | (None, Color::White, Field::WhitePiece(_))
                    | (None, Color::White, Field::WhiteKing) => visited = Some(safepos),
                    (None, Color::White, Field::BlackPiece(_))
                    | (None, Color::White, Field::BlackKing)
                    | (None, Color::Black, Field::WhitePiece(_))
                    | (None, Color::Black, Field::WhiteKing) => break,
                    (Some(visited), Color::White, Field::BlackPiece(piece))
                    | (Some(visited), Color::Black, Field::WhitePiece(piece))
                        if matches!((dir.is_diagonal(), piece), (_, Piece::Queen) | (true, Piece::Bishop) | (false, Piece::Rook)) =>
                    {
                        pcs.append((visited, *dir))
                    }
                    (Some(_visited), _, Field::BlackPiece(_))
                    | (Some(_visited), _, Field::WhitePiece(_))
                    | (Some(_visited), _, Field::WhiteKing)
                    | (Some(_visited), _, Field::BlackKing) => break,
                    _ => (),
                }
            }
        }
        pcs
    }

    pub fn update_potential_checks(&mut self) {
        for &c in &[Color::Black, Color::White] {
            self.get_king_mut(c).clean_check_map();
            for &(coord, dir) in self.get_potential_checks(self.get_king(c).coord, c).slice() {
                *self.get_king_mut(c).get_potential_check_mut(coord) = Some((coord, dir));
            }
        }
    }

    pub fn get_threatened_by<C: CommonCoord>(&self, coord: C, color: Color) -> bool {
        self.threat_mask
            .get(coord)
            .slice()
            .iter()
            .any(|&coord| self.get(coord).is_color_piece_include_king(color))
    }
}
