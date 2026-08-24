use chess::{Board, Color, Piece, Rank, Square};

const PASSED_PAWN_EG: [i32; 8] = [0, 10, 20, 35, 60, 100, 160, 0];

const KING_TROPISM_WEIGHT: i32 = 5;


const ROOK_BEHIND_PASSED_PAWN_BONUS: i32 = 15;

pub(crate) fn endgame_score(board: &Board) -> (i32, i32) {
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

    score += passed_pawn_score(board, Color::White, white_king, black_king);
    score -= passed_pawn_score(board, Color::Black, black_king, white_king);

    (0, score)
}

fn passed_pawn_score(board: &Board, color: Color, own_king: Square, enemy_king: Square) -> i32 {
    let own_pawns = *board.color_combined(color) & *board.pieces(Piece::Pawn);
    let rooks = *board.color_combined(color) & *board.pieces(Piece::Rook);

    let mut score = 0;

    for sq in own_pawns {
        if !is_passed_pawn(board, sq, color) {
            continue;
        }

        let rank_idx = sq.get_rank().to_index();
        let progress = match color {
            Color::White => rank_idx,
            Color::Black => 7 - rank_idx,
        };

        score += PASSED_PAWN_EG[progress];

        let promo_rank = match color {
            Color::White => 7,
            Color::Black => 0,
        };
        let promo_sq = Square::make_square(Rank::from_index(promo_rank), sq.get_file());

        let own_king_dist = distance(own_king, promo_sq);
        let enemy_king_dist = distance(enemy_king, promo_sq);

        score += (enemy_king_dist - own_king_dist) * KING_TROPISM_WEIGHT;

        for rook_sq in rooks {
            if rook_sq.get_file() != sq.get_file() {
                continue;
            }

            let behind = match color {
                Color::White => rook_sq.get_rank().to_index() < rank_idx,
                Color::Black => rook_sq.get_rank().to_index() > rank_idx,
            };

            if behind {
                score += ROOK_BEHIND_PASSED_PAWN_BONUS;
            }
        }
    }

    score
}

fn is_passed_pawn(board: &Board, sq: Square, color: Color) -> bool {
    let enemy_pawns = *board.color_combined(!color) & *board.pieces(Piece::Pawn);

    let file = sq.get_file().to_index() as i32;
    let rank = sq.get_rank().to_index() as i32;

    for enemy_sq in enemy_pawns {
        let ef = enemy_sq.get_file().to_index() as i32;
        let er = enemy_sq.get_rank().to_index() as i32;

        if (ef - file).abs() > 1 {
            continue;
        }

        let blocks = match color {
            Color::White => er > rank,
            Color::Black => er < rank,
        };

        if blocks {
            return false;
        }
    }

    true
}

fn distance(sq1: Square, sq2: Square) -> i32 {
    let f1 = sq1.get_file().to_index() as i32;
    let r1 = sq1.get_rank().to_index() as i32;
    let f2 = sq2.get_file().to_index() as i32;
    let r2 = sq2.get_rank().to_index() as i32;

    (f1 - f2).abs().max((r1 - r2).abs())
}