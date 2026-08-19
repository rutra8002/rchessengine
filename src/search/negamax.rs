use chess::Board;

use crate::{
    ordering::ordered_legal_moves,
};

use super::{
    quiescence::quiescence,
    transposition::Bound,
    Search,
    SearchStats,
};

const INF: i32 = i32::MAX / 2;
const MATE_SCORE: i32 = 900_000;

pub(crate) fn negamax(
    search: &mut Search,
    board: &Board,
    depth: u32,
    mut alpha: i32,
    beta: i32,
    ply: i32,
    stats: &mut SearchStats,
    history: &mut Vec<u64>,
) -> i32 {
    stats.nodes += 1;
    stats.check_time();

    if stats.stopped {
        return 0;
    }

    let hash = board.get_hash();

    let repetitions =
        history.iter().filter(|&&h| h == hash).count();

    if repetitions >= 2 {
        return 0;
    }

    if depth == 0 {
        return quiescence(
            board,
            alpha,
            beta,
            ply,
            stats,
        );
    }

    let original_alpha = alpha;

    let tt_move = if let Some(entry) =
        search.tt.probe(hash)
    {
        if entry.depth >= depth as i16 {
            stats.tt_hits += 1;

            match entry.bound {
                Bound::Exact => {
                    stats.tt_cutoffs += 1;
                    return entry.score;
                }

                Bound::Lower if entry.score >= beta => {
                    stats.tt_cutoffs += 1;
                    return entry.score;
                }

                Bound::Upper if entry.score <= alpha => {
                    stats.tt_cutoffs += 1;
                    return entry.score;
                }

                _ => {}
            }
        }

        entry.best_move
    } else {
        None
    };

    let moves = ordered_legal_moves(
        board,
        tt_move,
    );

    if moves.is_empty() {
        return if *board.checkers() != chess::EMPTY {
            -MATE_SCORE + ply
        } else {
            0
        };
    }

    let mut best = -INF;
    let mut best_move = None;

    for m in moves {
        let next = board.make_move_new(m);

        history.push(next.get_hash());

        let score = -negamax(
            search,
            &next,
            depth - 1,
            -beta,
            -alpha,
            ply + 1,
            stats,
            history,
        );

        history.pop();

        if stats.stopped {
            return 0;
        }

        if score > best {
            best = score;
            best_move = Some(m);
        }

        if score > alpha {
            alpha = score;
        }

        if alpha >= beta {
            break;
        }
    }

    if !stats.stopped {
        let bound = if best <= original_alpha {
            Bound::Upper
        } else if best >= beta {
            Bound::Lower
        } else {
            Bound::Exact
        };

        search.tt.store(
            hash,
            depth,
            best,
            bound,
            best_move,
        );
    }

    best
}