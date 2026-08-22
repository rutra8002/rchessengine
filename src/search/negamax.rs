use chess::{Board, ChessMove, Color, Piece, EMPTY};

use crate::ordering::ordered_legal_moves;

use super::{
    heuristics::record_killer,
    quiescence::quiescence,
    transposition::{score_from_tt, score_to_tt, Bound},
    Search, SearchStats, GameHistory,
};

const INF: i32 = i32::MAX / 2;
const MATE_SCORE: i32 = 67_000_000;

const NULL_MOVE_R: u32 = 2;
const NULL_MOVE_MIN_DEPTH: u32 = NULL_MOVE_R + 1;

const LMR_MIN_DEPTH: u32 = 3;
const LMR_MIN_MOVE_INDEX: usize = 3;

const CHECK_EXTENSION: u32 = 1;

const MAX_CHECK_EXTENSION_PLY: i32 = 64;

#[inline]
fn has_non_pawn_material(board: &Board, color: Color) -> bool {
    let pieces = *board.color_combined(color);
    let pawns_and_king = *board.pieces(Piece::Pawn) | *board.pieces(Piece::King);
    (pieces & !pawns_and_king).popcnt() > 0
}

pub(crate) fn negamax(
    search: &mut Search,
    board: &Board,
    depth: u32,
    mut alpha: i32,
    beta: i32,
    ply: i32,
    stats: &mut SearchStats,
    history: &mut GameHistory,
) -> i32 {
    stats.nodes += 1;
    stats.check_time();

    if stats.stopped {
        return 0;
    }

    let hash = board.get_hash();

    if history.count(hash) >= 3 {
        return 0;
    }

    if depth == 0 {
        return quiescence(board, alpha, beta, ply, stats);
    }

    let in_check = *board.checkers() != EMPTY;
    let original_alpha = alpha;

    let tt_move = if let Some(entry) = search.tt.probe(hash) {
        if entry.depth >= depth as i16 {
            stats.tt_hits += 1;

            let adjusted_score = score_from_tt(entry.score, ply);

            match entry.bound {
                Bound::Exact => {
                    stats.tt_cutoffs += 1;
                    return adjusted_score;
                }

                Bound::Lower if adjusted_score >= beta => {
                    stats.tt_cutoffs += 1;
                    return adjusted_score;
                }

                Bound::Upper if adjusted_score <= alpha => {
                    stats.tt_cutoffs += 1;
                    return adjusted_score;
                }

                _ => {}
            }
        }

        entry.best_move()
    } else {
        None
    };

    if !in_check
        && depth >= NULL_MOVE_MIN_DEPTH
        && beta.abs() < MATE_SCORE - 1000
        && has_non_pawn_material(board, board.side_to_move())
    {
        if let Some(null_board) = board.null_move() {
            let reduced_depth = depth - 1 - NULL_MOVE_R;

            history.push(null_board.get_hash());

            let null_score = -negamax(
                search,
                &null_board,
                reduced_depth,
                -beta,
                -beta + 1,
                ply + 1,
                stats,
                history,
            );

            history.pop();

            if stats.stopped {
                return 0;
            }

            if null_score >= beta {
                return beta;
            }
        }
    }

    let moves = {
        let side_to_move = board.side_to_move();

        let killers_here = search
            .killers
            .get(ply as usize)
            .copied()
            .unwrap_or([None, None]);

        let history_table = &search.history_table;

        ordered_legal_moves(board, tt_move, killers_here, |m| {
            history_table.score(side_to_move, m)
        })
    };

    if moves.is_empty() {
        return if in_check {
            -MATE_SCORE + ply
        } else {
            0
        };
    }

    let mut best = -INF;
    let mut best_move = None;
    let mut quiets_tried: Vec<ChessMove> = Vec::new();

    for (move_index, m) in moves.into_iter().enumerate() {
        let is_capture = board.piece_on(m.get_dest()).is_some();

        if !is_capture {
            quiets_tried.push(m);
        }

        let next = board.make_move_new(m);
        let gives_check = *next.checkers() != EMPTY;

        let extension: u32 =
            if gives_check && ply < MAX_CHECK_EXTENSION_PLY {
                CHECK_EXTENSION
            } else {
                0
            };

        history.push(next.get_hash());

        let score = if move_index == 0 {
            -negamax(
                search,
                &next,
                depth - 1 + extension,
                -beta,
                -alpha,
                ply + 1,
                stats,
                history,
            )
        } else {
            let reduce = !is_capture
                && !gives_check
                && !in_check
                && depth >= LMR_MIN_DEPTH
                && move_index >= LMR_MIN_MOVE_INDEX;

            let searched_depth = if reduce {
                (depth - 1).saturating_sub(1)
            } else {
                depth - 1 + extension
            };

            let mut score = -negamax(
                search,
                &next,
                searched_depth,
                -alpha - 1,
                -alpha,
                ply + 1,
                stats,
                history,
            );

            if score > alpha && searched_depth < depth - 1 + extension {
                score = -negamax(
                    search,
                    &next,
                    depth - 1 + extension,
                    -alpha - 1,
                    -alpha,
                    ply + 1,
                    stats,
                    history,
                );
            }

            if score > alpha && score < beta {
                score = -negamax(
                    search,
                    &next,
                    depth - 1 + extension,
                    -beta,
                    -alpha,
                    ply + 1,
                    stats,
                    history,
                );
            }

            score
        };

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

        if bound == Bound::Lower {
            if let Some(bm) = best_move {
                let bm_is_capture = board.piece_on(bm.get_dest()).is_some();

                if !bm_is_capture {
                    if let Some(killers_here) =
                        search.killers.get_mut(ply as usize)
                    {
                        record_killer(killers_here, bm);
                    }

                    search.history_table.update(
                        board.side_to_move(),
                        bm,
                        &quiets_tried,
                        depth,
                    );
                }
            }
        }

        search.tt.store(
            hash,
            depth,
            score_to_tt(best, ply),
            bound,
            best_move,
        );
    }

    best
}