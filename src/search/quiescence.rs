use chess::{
    Board,
    BoardStatus,
    ChessMove,
    MoveGen,
    EMPTY,
};

use crate::{
    evaluation::{evaluate_relative, piece_value},
    ordering::move_order_score,
};

use super::{SearchStats, MATE_SCORE};

const DELTA_MARGIN: i32 = 200;

pub(crate) fn quiescence(
    board: &Board,
    mut alpha: i32,
    beta: i32,
    ply: i32,
    stats: &mut SearchStats,
) -> i32 {
    stats.nodes += 1;
    stats.check_time();

    if stats.stopped {
        return 0;
    }

    match board.status() {
        BoardStatus::Checkmate => {
            return -MATE_SCORE + ply;
        }

        BoardStatus::Stalemate => {
            return 0;
        }

        BoardStatus::Ongoing => {}
    }

    let in_check = *board.checkers() != EMPTY;

    let stand_pat = if !in_check {
        let sp = evaluate_relative(board);

        if sp >= beta {
            return beta;
        }

        if sp > alpha {
            alpha = sp;
        }

        Some(sp)
    } else {
        None
    };

    let mut moves: Vec<ChessMove> = if in_check {
        MoveGen::new_legal(board).collect()
    } else {
        let targets = *board.color_combined(!board.side_to_move());
        let mut movegen = MoveGen::new_legal(board);

        movegen.set_iterator_mask(targets);
        movegen.collect()
    };

    let mut scored: Vec<(ChessMove, i32)> = moves
        .drain(..)
        .map(|m| {
            let score = move_order_score(board, m);
            (m, score)
        })
        .collect();

    scored.sort_unstable_by(|a, b| b.1.cmp(&a.1));

    for (m, _) in scored {
        if let Some(sp) = stand_pat {
            if m.get_promotion().is_none() {
                if let Some(victim) = board.piece_on(m.get_dest()) {
                    let optimistic = sp + piece_value(victim) + DELTA_MARGIN;

                    if optimistic <= alpha {
                        continue;
                    }
                }
            }
        }

        let next = board.make_move_new(m);

        let score = -quiescence(
            &next,
            -beta,
            -alpha,
            ply + 1,
            stats,
        );

        if stats.stopped {
            return 0;
        }

        if score >= beta {
            return beta;
        }

        if score > alpha {
            alpha = score;
        }
    }

    alpha
}