use chess::{Board, Color, Piece, Square};

pub(crate) fn positional_principles_score(board: &Board) -> (i32, i32) {
    let mut mg_score = 0;
    let mut eg_score = 0;

    let white_pawns = *board.color_combined(Color::White) & *board.pieces(Piece::Pawn);
    let black_pawns = *board.color_combined(Color::Black) & *board.pieces(Piece::Pawn);

    let central_squares = [Square::E4, Square::D4, Square::E5, Square::D5];
    let extended_center = [Square::C4, Square::F4, Square::C5, Square::F5];

    for sq in central_squares {
        let square_bb = chess::BitBoard::from_square(sq);
        if (white_pawns & square_bb).popcnt() > 0 {
            mg_score += 15;
            eg_score += 15;
        }
        if (black_pawns & square_bb).popcnt() > 0 {
            mg_score -= 15;
            eg_score -= 15;
        }
    }

    for sq in extended_center {
        let square_bb = chess::BitBoard::from_square(sq);
        if (white_pawns & square_bb).popcnt() > 0 {
            mg_score += 5;
            eg_score += 5;
        }
        if (black_pawns & square_bb).popcnt() > 0 {
            mg_score -= 5;
            eg_score -= 5;
        }
    }

    let white_king_sq = board.king_square(Color::White);
    let black_king_sq = board.king_square(Color::Black);

    if white_king_sq == Square::G1 || white_king_sq == Square::C1 {
        mg_score += 40;
    }

    if black_king_sq == Square::G8 || black_king_sq == Square::C8 {
        mg_score -= 40;
    }

    (mg_score, eg_score)
}