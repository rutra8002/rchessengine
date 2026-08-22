use chess::{BitBoard, Board, Color, Piece, Square};
use crate::evaluation::piece_value;

#[inline]
fn pawn_attacks_from(square: Square, color: Color) -> BitBoard {
    let file = square.get_file().to_index() as i32;
    let rank = square.get_rank().to_index() as i32;
    let mut bits = 0u64;
    let direction = match color {
        Color::White => 1,
        Color::Black => -1,
    };
    let target_rank = rank + direction;
    if !(0..8).contains(&target_rank) { return BitBoard(0); }
    if file > 0 { bits |= 1u64 << (target_rank * 8 + file - 1); }
    if file < 7 { bits |= 1u64 << (target_rank * 8 + file + 1); }
    BitBoard(bits)
}

pub(crate) fn get_least_valuable_attacker(
    board: &Board,
    target: Square,
    color: Color,
    occupied: BitBoard,
) -> Option<(Piece, Square)> {
    let color_mask = *board.color_combined(color) & occupied;

    let pawns = *board.pieces(Piece::Pawn) & color_mask;
    if pawns.popcnt() > 0 {
        let attackers = pawns & pawn_attacks_from(target, !color);
        if let Some(sq) = attackers.into_iter().next() { return Some((Piece::Pawn, sq)); }
    }

    let knights = *board.pieces(Piece::Knight) & color_mask;
    if knights.popcnt() > 0 {
        let attackers = knights & chess::get_knight_moves(target);
        if let Some(sq) = attackers.into_iter().next() { return Some((Piece::Knight, sq)); }
    }

    let b_q = (*board.pieces(Piece::Bishop) | *board.pieces(Piece::Queen)) & color_mask;
    if b_q.popcnt() > 0 {
        let attackers = b_q & chess::get_bishop_moves(target, occupied);
        if attackers.popcnt() > 0 {
            let bishops = attackers & *board.pieces(Piece::Bishop);
            if let Some(sq) = bishops.into_iter().next() { return Some((Piece::Bishop, sq)); }
            else if let Some(sq) = attackers.into_iter().next() { return Some((Piece::Queen, sq)); }
        }
    }

    let r_q = (*board.pieces(Piece::Rook) | *board.pieces(Piece::Queen)) & color_mask;
    if r_q.popcnt() > 0 {
        let attackers = r_q & chess::get_rook_moves(target, occupied);
        if attackers.popcnt() > 0 {
            let rooks = attackers & *board.pieces(Piece::Rook);
            if let Some(sq) = rooks.into_iter().next() { return Some((Piece::Rook, sq)); }
            else if let Some(sq) = attackers.into_iter().next() { return Some((Piece::Queen, sq)); }
        }
    }

    let kings = *board.pieces(Piece::King) & color_mask;
    if kings.popcnt() > 0 {
        let attackers = kings & chess::get_king_moves(target);
        if let Some(sq) = attackers.into_iter().next() { return Some((Piece::King, sq)); }
    }

    None
}

pub(crate) fn see(board: &Board, target: Square, target_piece: Piece, attacker_sq: Square, attacker_piece: Piece) -> i32 {
    let mut gain = [0; 32];
    let mut d = 0;
    let mut occupied = *board.combined();
    let mut color = board.color_on(attacker_sq).unwrap();

    gain[d] = piece_value(target_piece);
    let current_attacker_sq = attacker_sq;
    let mut current_attacker_piece = attacker_piece;

    occupied ^= BitBoard::from_square(current_attacker_sq);

    loop {
        d += 1;
        color = !color;
        if let Some((piece, sq)) = get_least_valuable_attacker(board, target, color, occupied) {
            gain[d] = piece_value(current_attacker_piece) - gain[d - 1];
            current_attacker_piece = piece;
            occupied ^= BitBoard::from_square(sq);
        } else {
            break;
        }
    }

    while d > 1 {
        d -= 1;
        gain[d - 1] = gain[d - 1].max(-gain[d]);
    }
    gain[0]
}