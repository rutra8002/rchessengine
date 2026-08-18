use chess::{Board, ChessMove, MoveGen};

use crate::evaluation::piece_value;

pub(crate) fn move_order_score(
    board: &Board,
    m: ChessMove,
) -> i32 {
    if let Some(victim) = board.piece_on(m.get_dest()) {
        let attacker_value = board
            .piece_on(m.get_source())
            .map(piece_value)
            .unwrap_or(0);

        10_000 + piece_value(victim) * 10 - attacker_value
    } else {
        0
    }
}

pub(crate) fn ordered_legal_moves(
    board: &Board,
    tt_move: Option<ChessMove>,
) -> Vec<ChessMove> {
    let mut moves: Vec<ChessMove> =
        MoveGen::new_legal(board).collect();

    moves.sort_unstable_by_key(|&m| {
        let tt_bonus = if Some(m) == tt_move {
            1_000_000
        } else {
            0
        };

        -(tt_bonus + move_order_score(board, m))
    });

    moves
}