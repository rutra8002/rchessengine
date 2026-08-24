use chess::{BitBoard, Board, Color, File, Piece};

const ISOLATED_PAWN_MG: i32 = 12;
const ISOLATED_PAWN_EG: i32 = 18;

const DOUBLED_PAWN_MG: i32 = 10;
const DOUBLED_PAWN_EG: i32 = 20;

const ROOK_OPEN_FILE_MG: i32 = 20;
const ROOK_OPEN_FILE_EG: i32 = 10;

const ROOK_SEMI_OPEN_FILE_MG: i32 = 10;
const ROOK_SEMI_OPEN_FILE_EG: i32 = 5;

#[inline]
fn file_mask(file: File) -> BitBoard {
    let mut bits = 0u64;

    for rank in 0..8 {
        bits |= 1u64 << (rank * 8 + file.to_index());
    }

    BitBoard(bits)
}

fn pawn_weaknesses_for(board: &Board, color: Color) -> (i32, i32) {
    let pawns = *board.color_combined(color) & *board.pieces(Piece::Pawn);

    let mut file_counts = [0u32; 8];
    for sq in pawns {
        file_counts[sq.get_file().to_index()] += 1;
    }

    let mut mg = 0;
    let mut eg = 0;

    for sq in pawns {
        let file_idx = sq.get_file().to_index();

        let has_left = file_idx > 0 && file_counts[file_idx - 1] > 0;
        let has_right = file_idx < 7 && file_counts[file_idx + 1] > 0;

        if !has_left && !has_right {
            mg -= ISOLATED_PAWN_MG;
            eg -= ISOLATED_PAWN_EG;
        }
    }

    for &count in file_counts.iter() {
        if count > 1 {
            let extra = count as i32 - 1;
            mg -= DOUBLED_PAWN_MG * extra;
            eg -= DOUBLED_PAWN_EG * extra;
        }
    }

    (mg, eg)
}

fn rook_activity_for(board: &Board, color: Color) -> (i32, i32) {
    let own_pawns = *board.color_combined(color) & *board.pieces(Piece::Pawn);
    let enemy_pawns = *board.color_combined(!color) & *board.pieces(Piece::Pawn);
    let rooks = *board.color_combined(color) & *board.pieces(Piece::Rook);

    let mut mg = 0;
    let mut eg = 0;

    for sq in rooks {
        let mask = file_mask(sq.get_file());

        let own_on_file = (own_pawns & mask).popcnt();
        let enemy_on_file = (enemy_pawns & mask).popcnt();

        if own_on_file == 0 && enemy_on_file == 0 {
            mg += ROOK_OPEN_FILE_MG;
            eg += ROOK_OPEN_FILE_EG;
        } else if own_on_file == 0 {
            mg += ROOK_SEMI_OPEN_FILE_MG;
            eg += ROOK_SEMI_OPEN_FILE_EG;
        }
    }

    (mg, eg)
}

pub(crate) fn structure_score(board: &Board) -> (i32, i32) {
    let (white_pawn_mg, white_pawn_eg) = pawn_weaknesses_for(board, Color::White);
    let (black_pawn_mg, black_pawn_eg) = pawn_weaknesses_for(board, Color::Black);

    let (white_rook_mg, white_rook_eg) = rook_activity_for(board, Color::White);
    let (black_rook_mg, black_rook_eg) = rook_activity_for(board, Color::Black);

    let mg = (white_pawn_mg - black_pawn_mg) + (white_rook_mg - black_rook_mg);
    let eg = (white_pawn_eg - black_pawn_eg) + (white_rook_eg - black_rook_eg);

    (mg, eg)
}