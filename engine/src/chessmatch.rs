use crate::board::{Board, Color};
use crate::moves::{LongMoveList, Move};
use std::sync::RwLock;

#[derive(Debug, Clone, Copy)]
pub enum MatchResult {
    WhiteWins,
    BlackWins,
    Stalemate,
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

    pub fn do_move(&self, id: u32, mv: Move) {
        if let (Some(v), Some(i)) = (
            self.boards.write().unwrap().get_mut(id as usize),
            self.infos.write().unwrap().get_mut(id as usize),
        ) {
            v.do_move(mv);
            i.color = !i.color;
            v.update_aggressors(i.color);
            let mut moves = LongMoveList::new();
            v.enumerate_all_moves_by(i.color, &mut moves);
            if moves.is_empty() {
                i.result = Some(if v.get_king(i.color).aggressors.is_empty() {
                    MatchResult::Stalemate
                } else if let Color::White = i.color {
                    MatchResult::BlackWins
                } else {
                    MatchResult::WhiteWins
                })
            }
        }
    }

    pub fn get_info(&self, id: u32) -> Option<MatchInfos<E>> {
        self.infos.read().unwrap().get(id as usize).cloned()
    }
}
