use chess::{Board, ChessMove, MoveGen};

use crate::evaluation::piece_value;

const TT_BONUS: i32 = 1_000_000_000;
const CAPTURE_BASE: i32 = 100_000_000;
const KILLER_1_BONUS: i32 = 90_000;
const KILLER_2_BONUS: i32 = 80_000;

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
    killers: [Option<ChessMove>; 2],
    history_score: impl Fn(ChessMove) -> i32,
) -> Vec<ChessMove> {
    let mut moves: Vec<ChessMove> =
        MoveGen::new_legal(board).collect();

    moves.sort_unstable_by_key(|&m| {
        if Some(m) == tt_move {
            return -TT_BONUS;
        }

        let is_capture = board.piece_on(m.get_dest()).is_some();

        let score = if is_capture {
            CAPTURE_BASE + move_order_score(board, m)
        } else if Some(m) == killers[0] {
            KILLER_1_BONUS
        } else if Some(m) == killers[1] {
            KILLER_2_BONUS
        } else {
            history_score(m)
        };

        -score
    });

    moves
}