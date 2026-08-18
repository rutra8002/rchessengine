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