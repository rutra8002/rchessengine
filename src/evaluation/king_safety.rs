use chess::{Board, Color, Piece, Rank, Square, BitBoard};

pub(crate) fn king_safety_score(board: &Board) -> i32 {
    let white_safety = evaluate_pawn_shield(board, Color::White);
    let black_safety = evaluate_pawn_shield(board, Color::Black);

    white_safety - black_safety
}

fn evaluate_pawn_shield(board: &Board, color: Color) -> i32 {
    let mut king_sq = None;
    for sq in *board.combined() {
        if board.piece_on(sq) == Some(Piece::King) && board.color_on(sq) == Some(color) {
            king_sq = Some(sq);
            break;
        }
    }

    let king_sq = match king_sq {
        Some(sq) => sq,
        None => return 0,
    };

    let file_idx = king_sq.get_file().to_index();
    let friendly_pawns = *board.color_combined(color) & *board.pieces(Piece::Pawn);

    let shield_rank = match color {
        Color::White => Rank::Second,
        Color::Black => Rank::Seventh,
    };

    let mut shield_score = 0;

    for f_offset in [-1i32, 0, 1] {
        let target_file_idx = file_idx as i32 + f_offset;
        if (0..8).contains(&target_file_idx) {
            let file = chess::File::from_index(target_file_idx as usize);
            let square = Square::make_square(shield_rank, file);
            let square_bb = BitBoard::from_square(square);

            if (friendly_pawns & square_bb).popcnt() > 0 {
                shield_score += 25;
            }
        }
    }

    shield_score
}