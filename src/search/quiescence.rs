use chess::{
    Board,
    BoardStatus,
    ChessMove,
    MoveGen,
};

use crate::{
    evaluation::evaluate_relative,
    ordering::move_order_score,
};

use super::SearchStats;

const MATE_SCORE: i32 = 900_000;

pub(crate) fn quiescence(
    board: &Board,
    mut alpha: i32,
    beta: i32,
    ply: i32,
    stats: &mut SearchStats,
) -> i32 {
    stats.nodes += 1;

    match board.status() {
        BoardStatus::Checkmate => {
            return -MATE_SCORE + ply;
        }

        BoardStatus::Stalemate => {
            return 0;
        }

        BoardStatus::Ongoing => {}
    }

    let stand_pat = evaluate_relative(board);

    if stand_pat >= beta {
        return beta;
    }

    if stand_pat > alpha {
        alpha = stand_pat;
    }

    let targets =
        *board.color_combined(!board.side_to_move());

    let mut movegen = MoveGen::new_legal(board);

    movegen.set_iterator_mask(targets);

    let mut captures: Vec<ChessMove> =
        movegen.collect();

    captures.sort_unstable_by_key(|&m| {
        -move_order_score(board, m)
    });

    for m in captures {
        let next = board.make_move_new(m);

        let score = -quiescence(
            &next,
            -beta,
            -alpha,
            ply + 1,
            stats,
        );

        if score >= beta {
            return beta;
        }

        if score > alpha {
            alpha = score;
        }
    }

    alpha
}