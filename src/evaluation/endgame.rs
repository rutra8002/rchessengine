use chess::{Board, Color, Piece, Square};

pub(crate) fn phase_aware_evaluation(board: &Board) -> i32 {
    let mut non_pawn_material = 0;
    for sq in *board.combined() {
        if let Some(piece) = board.piece_on(sq) {
            match piece {
                Piece::Knight | Piece::Bishop => non_pawn_material += 1,
                Piece::Rook => non_pawn_material += 2,
                Piece::Queen => non_pawn_material += 4,
                _ => {}
            }
        }
    }

    if non_pawn_material <= 6 {
        evaluate_endgame(board)
    } else {
        0
    }
}

fn evaluate_endgame(board: &Board) -> i32 {
    let mut score = 0;

    let white_king = board.king_square(Color::White);
    let black_king = board.king_square(Color::Black);

    let center_squares = [Square::D4, Square::E4, Square::D5, Square::E5];

    let white_king_distance = center_squares.iter()
        .map(|&sq| distance(white_king, sq))
        .min().unwrap_or(0);

    let black_king_distance = center_squares.iter()
        .map(|&sq| distance(black_king, sq))
        .min().unwrap_or(0);

    score += (4 - white_king_distance) * 15;
    score -= (4 - black_king_distance) * 15;

    for sq in *board.combined() {
        if let Some(Piece::Pawn) = board.piece_on(sq) {
            let color = board.color_on(sq).unwrap();
            let rank_idx = sq.get_rank().to_index() as i32;

            if color == Color::White {
                score += rank_idx * 10;
            } else {
                score -= (7 - rank_idx) * 10;
            }
        }
    }

    score
}

fn distance(sq1: Square, sq2: Square) -> i32 {
    let f1 = sq1.get_file().to_index() as i32;
    let r1 = sq1.get_rank().to_index() as i32;
    let f2 = sq2.get_file().to_index() as i32;
    let r2 = sq2.get_rank().to_index() as i32;

    (f1 - f2).abs().max((r1 - r2).abs())
}