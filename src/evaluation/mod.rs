mod material;
mod mobility;

use chess::Board;

pub(crate) use material::piece_value;

pub(crate) fn evaluate(board: &Board) -> i32 {
    let material = material::material_score(board);
    let mobility = mobility::mobility_score(board);

    material + 2 * mobility
}

pub(crate) fn evaluate_relative(board: &Board) -> i32 {
    let score = evaluate(board);

    if board.side_to_move() == chess::Color::White {
        score
    } else {
        -score
    }
}