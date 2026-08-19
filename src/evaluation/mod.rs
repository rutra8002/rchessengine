mod material;
mod mobility;
mod king_safety;
mod opening;

use chess::Board;

pub(crate) use material::piece_value;

pub(crate) fn evaluate(board: &Board) -> i32 {
    let material = material::material_score(board);
    let mobility = mobility::mobility_score(board);
    let king_safety = king_safety::king_safety_score(board);
    let positional = opening::positional_principles_score(board);
    material + mobility + king_safety + positional
}

pub(crate) fn evaluate_relative(board: &Board) -> i32 {
    let score = evaluate(board);

    if board.side_to_move() == chess::Color::White {
        score
    } else {
        -score
    }
}