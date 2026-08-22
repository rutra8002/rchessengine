use chess::{
    get_bishop_moves,
    get_king_moves,
    get_knight_moves,
    get_rook_moves,
    Board,
    Color,
    Piece,
    EMPTY,
};

pub(crate) fn mobility_score(board: &Board) -> (i32, i32) {
    let occupied = *board.combined();

    let white_pieces = *board.color_combined(Color::White);
    let black_pieces = *board.color_combined(Color::Black);

    let mut white_mobility = 0i32;
    let mut black_mobility = 0i32;

    for sq in occupied {
        let piece = match board.piece_on(sq) {
            Some(piece) => piece,
            None => continue,
        };

        let color = board.color_on(sq).unwrap();

        let own_pieces = if color == Color::White {
            white_pieces
        } else {
            black_pieces
        };

        let attacks = match piece {
            Piece::Knight => get_knight_moves(sq),
            Piece::Bishop => get_bishop_moves(sq, occupied),
            Piece::Rook => get_rook_moves(sq, occupied),
            Piece::Queen => get_bishop_moves(sq, occupied) | get_rook_moves(sq, occupied),
            Piece::King => get_king_moves(sq),
            Piece::Pawn => EMPTY,
        };

        let count = (attacks & !own_pieces).popcnt() as i32;

        if color == Color::White {
            white_mobility += count;
        } else {
            black_mobility += count;
        }
    }

    let score = white_mobility - black_mobility;

    (score, score)
}