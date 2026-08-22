mod material;
mod mobility;
mod king_safety;
mod opening;
mod endgame;
pub mod pst;

use chess::Board;

pub(crate) use material::piece_value;

pub(crate) fn evaluate(board: &Board) -> i32 {
    let material = material::material_score(board);
    let (mob_mg, mob_eg) = mobility::mobility_score(board);
    let (ks_mg, ks_eg) = king_safety::king_safety_score(board);
    let (pos_mg, pos_eg) = opening::positional_principles_score(board);
    let (end_mg, end_eg) = endgame::endgame_score(board);
    let (pst_mg, pst_eg, phase) = pst::pst_score(board);

    let mg_total = material + (2 * mob_mg) + ks_mg + pos_mg + end_mg + pst_mg;

    let eg_total = material + (2 * mob_eg) + ks_eg + pos_eg + end_eg + pst_eg;

    (mg_total * phase + eg_total * (pst::TOTAL_PHASE - phase)) / pst::TOTAL_PHASE
}

pub(crate) fn evaluate_relative(board: &Board) -> i32 {
    let score = evaluate(board);

    if board.side_to_move() == chess::Color::White {
        score
    } else {
        -score
    }
}