use crate::board::{Board, Color};
use crate::moves::{LongMoveList, Move};

pub struct DecisionMaker {
    board: Board,
}

impl DecisionMaker {
    pub fn from_board(board: Board) -> Self {
        Self { board }
    }

    pub fn get(&self, color: Color) -> Option<Move> {
        let mut moves = LongMoveList::new();
        self.board.enumerate_all_moves_by(color, &mut moves);
        moves
            .slice()
            .first()
            .and_then(|i| i.slice().first().cloned())
    }
}
