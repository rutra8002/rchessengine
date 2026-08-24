use chess::{Board, Color, Piece};

pub(crate) fn piece_value(piece: Piece) -> i32 {
    match piece {
        Piece::Pawn => 100,
        Piece::Knight => 300,
        Piece::Bishop => 330,
        Piece::Rook => 500,
        Piece::Queen => 900,
        Piece::King => 0,
    }
}

const BISHOP_PAIR_BONUS: i32 = 30;

pub(crate) fn material_score(board: &Board) -> i32 {
    let mut score = 0;

    for sq in *board.combined() {
        if let Some(piece) = board.piece_on(sq) {
            let value = piece_value(piece);
            let color = board.color_on(sq).unwrap();

            score += if color == Color::White {
                value
            } else {
                -value
            };
        }
    }

    score
}

pub(crate) fn bishop_pair_score(board: &Board) -> i32 {
    let white_bishops =
        (*board.color_combined(Color::White) & *board.pieces(Piece::Bishop)).popcnt();
    let black_bishops =
        (*board.color_combined(Color::Black) & *board.pieces(Piece::Bishop)).popcnt();

    let mut score = 0;

    if white_bishops >= 2 {
        score += BISHOP_PAIR_BONUS;
    }

    if black_bishops >= 2 {
        score -= BISHOP_PAIR_BONUS;
    }

    score
}