mod negamax;
mod quiescence;
mod transposition;

use chess::{Board, ChessMove};

use crate::ordering::ordered_legal_moves;

use transposition::{
    Bound,
    TranspositionTable,
};

pub const DEFAULT_DEPTH: u32 = 7;

const INF: i32 = i32::MAX / 2;

pub struct SearchStats {
    pub nodes: u64,
    pub tt_hits: u64,
    pub tt_cutoffs: u64,
}

pub struct Search {
    tt: TranspositionTable,
}

impl Search {
    pub fn new() -> Self {
        Self {
            tt: TranspositionTable::new(),
        }
    }

    pub fn clear_tt(&mut self) {
        self.tt.clear();
    }

    pub fn search_best_move(
        &mut self,
        board: &Board,
        depth: u32,
        history: &mut Vec<u64>,
    ) -> (
        Option<ChessMove>,
        i32,
        u64,
        u64,
        u64,
    ) {
        let mut stats = SearchStats {
            nodes: 0,
            tt_hits: 0,
            tt_cutoffs: 0,
        };

        let mut alpha = -INF;
        let beta = INF;

        let mut best_move = None;
        let mut best_score = -INF;

        let hash = board.get_hash();

        let tt_move = self
            .tt
            .probe(hash)
            .and_then(|entry| entry.best_move);

        let moves = ordered_legal_moves(
            board,
            tt_move,
        );

        for m in moves {
            let next = board.make_move_new(m);

            history.push(next.get_hash());

            let score = -negamax::negamax(
                self,
                &next,
                depth.saturating_sub(1),
                -beta,
                -alpha,
                1,
                &mut stats,
                history,
            );

            history.pop();

            if score > best_score || best_move.is_none() {
                best_score = score;
                best_move = Some(m);
            }

            if best_score > alpha {
                alpha = best_score;
            }
        }

        self.tt.store(
            hash,
            depth,
            best_score,
            Bound::Exact,
            best_move,
        );

        (
            best_move,
            best_score,
            stats.nodes,
            stats.tt_hits,
            stats.tt_cutoffs,
        )
    }
}