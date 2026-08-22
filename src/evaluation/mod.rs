mod material;
mod mobility;
mod king_safety;
mod opening;
mod endgame;
pub mod pst;
pub mod see;

use chess::{BitBoard, Board, Color, Piece, Square};

pub(crate) use material::piece_value;

#[inline]
fn pawn_attacks_to(square: Square, color: Color) -> BitBoard {
    let file = square.get_file().to_index() as i32;
    let rank = square.get_rank().to_index() as i32;
    let mut bits = 0u64;
    let direction = match color {
        Color::White => -1,
        Color::Black => 1,
    };
    let pawn_rank = rank + direction;
    if !(0..8).contains(&pawn_rank) {
        return BitBoard(0);
    }
    if file > 0 {
        bits |= 1u64 << (pawn_rank * 8 + file - 1);
    }
    if file < 7 {
        bits |= 1u64 << (pawn_rank * 8 + file + 1);
    }
    BitBoard(bits)
}

pub(crate) fn hanging_piece_penalty(board: &Board) -> i32 {
    let mut score = 0;

    let white_pieces = *board.color_combined(Color::White) & !*board.pieces(Piece::Pawn) & !*board.pieces(Piece::King);
    let black_pawns = *board.color_combined(Color::Black) & *board.pieces(Piece::Pawn);
    for sq in white_pieces {
        let piece = board.piece_on(sq).unwrap();
        if (pawn_attacks_to(sq, Color::White) & black_pawns).popcnt() > 0 {
            score -= piece_value(piece) / 2;
        }
    }

    let black_pieces = *board.color_combined(Color::Black) & !*board.pieces(Piece::Pawn) & !*board.pieces(Piece::King);
    let white_pawns = *board.color_combined(Color::White) & *board.pieces(Piece::Pawn);
    for sq in black_pieces {
        let piece = board.piece_on(sq).unwrap();
        if (pawn_attacks_to(sq, Color::Black) & white_pawns).popcnt() > 0 {
            score += piece_value(piece) / 2;
        }
    }

    score
}

pub(crate) fn evaluate(board: &Board) -> i32 {
    let material = material::material_score(board);
    let (mob_mg, mob_eg) = mobility::mobility_score(board);
    let (ks_mg, ks_eg) = king_safety::king_safety_score(board);
    let (pos_mg, pos_eg) = opening::positional_principles_score(board);
    let (end_mg, end_eg) = endgame::endgame_score(board);
    let (pst_mg, pst_eg, phase) = pst::pst_score(board);
    let threats = hanging_piece_penalty(board);

    let mg_total = material + (2 * mob_mg) + ks_mg + pos_mg + end_mg + pst_mg + threats;
    let eg_total = material + (2 * mob_eg) + ks_eg + pos_eg + end_eg + pst_eg + threats;

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