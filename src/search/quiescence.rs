use chess::{
    Board,
    BoardStatus,
    ChessMove,
    MoveGen,
    EMPTY,
};
use crate::{
    evaluation::{evaluate_relative, piece_value, see::see},
    ordering::move_order_score,
};

use super::{SearchStats, MATE_SCORE};

const DELTA_MARGIN: i32 = 200;
const MAX_QS_PLY: i32 = 100;

pub(crate) fn quiescence(
    board: &Board,
    mut alpha: i32,
    beta: i32,
    ply: i32,
    stats: &mut SearchStats,
) -> i32 {
    stats.nodes += 1;
    stats.check_time();
    if stats.stopped || ply >= MAX_QS_PLY {
        return evaluate_relative(board);
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

    moves.sort_unstable_by(|a, b| {
        let score_a = move_order_score(board, *a);
        let score_b = move_order_score(board, *b);
        score_b.cmp(&score_a)
    });

    for m in moves {
        if let Some(sp) = stand_pat {
            if m.get_promotion().is_none() {
                if let Some(victim) = board.piece_on(m.get_dest()) {
                    let optimistic = sp + piece_value(victim) + DELTA_MARGIN;

                    if optimistic <= alpha {
                        continue;
                    }
                    let attacker = board.piece_on(m.get_source()).unwrap();
                    let skip_see = piece_value(attacker) <= piece_value(victim);
                    if !skip_see {
                        if see(board, m.get_dest(), victim, m.get_source(), attacker) < 0 {
                            continue;
                        }
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