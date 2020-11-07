use crate::board::{Board, Color};
use crate::moves::{LongMoveList, Move};
use std::sync::RwLock;

#[derive(Debug, Clone, Copy)]
pub enum MatchResult {
    WhiteWins,
    BlackWins,
    Stalemate,
}

pub trait Opponent {
    fn decide<'m>(&mut self, board: &mut Board, moves: &'m LongMoveList, color: Color) -> &'m Move;
}

fn run_match<OW: Opponent, OB: Opponent>(mut white: OW, mut black: OB) -> MatchResult {
    let mut board = Board::new();
    let mut color = Color::White;
    let mut moves = LongMoveList::new();
    loop {
        board.update_aggressors(color);
        moves.clear();
        board.enumerate_all_moves_by(color, &mut moves);
        if moves.is_empty() {
            return if board.get_king(color).aggressors.is_empty() {
                MatchResult::Stalemate
            } else {
                match color {
                    Color::White => MatchResult::WhiteWins,
                    Color::Black => MatchResult::BlackWins,
                }
            };
        }
        let mv = *match color {
            Color::White => white.decide(&mut board, &moves, color),
            Color::Black => black.decide(&mut board, &moves, color),
        };
        board.do_move(mv);
        color = !color;
    }
}

#[derive(Clone)]
pub struct MatchInfos<E: Clone> {
    pub result: Option<MatchResult>,
    pub color: Color,
    pub extra: E,
}

pub struct MatchRegistry<E: Clone> {
    empty_slots: RwLock<Vec<u32>>,
    boards: RwLock<Vec<Board>>,
    infos: RwLock<Vec<MatchInfos<E>>>,
}

impl<E: Clone> Default for MatchRegistry<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E: Clone> MatchRegistry<E> {
    pub fn new() -> Self {
        Self {
            empty_slots: RwLock::new(vec![]),
            boards: RwLock::new(vec![]),
            infos: RwLock::new(vec![]),
        }
    }

    pub fn create_match(&self, extra: E) -> u32 {
        let mut boards = self.boards.write().unwrap();
        let mut infos = self.infos.write().unwrap();
        let (board, info) = (
            Board::new(),
            MatchInfos {
                result: None,
                color: Color::White,
                extra,
            },
        );
        (if self.empty_slots.read().unwrap().is_empty() {
            boards.push(board);
            infos.push(info);
            boards.len() - 1
        } else {
            let id = self.empty_slots.write().unwrap().pop().unwrap() as usize;
            boards[id] = board;
            infos[id] = info;
            id
        }) as u32
    }

    pub fn get_board(&self, id: u32) -> Option<Board> {
        self.boards.read().unwrap().get(id as usize).cloned()
    }

    pub fn get_infos(&self, id: u32) -> Option<MatchInfos<E>> {
        self.infos.read().unwrap().get(id as usize).cloned()
    }
}
