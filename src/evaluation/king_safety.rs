use chess::{BitBoard, Board, Color, File, Piece, Square};

const PAWN_SHIELD_BONUS: i32 = 18;
const MISSING_SHIELD_PENALTY: i32 = 12;

const OPEN_FILE_PENALTY: i32 = 22;
const SEMI_OPEN_FILE_PENALTY: i32 = 10;

const KING_ATTACK_BONUS: i32 = 8;
const KING_ZONE_ATTACK_BONUS: i32 = 4;

const CENTER_KING_PENALTY: i32 = 18;
const CASTLED_KING_BONUS: i32 = 12;


#[inline]
fn file_mask(file: File) -> BitBoard {
    let mut bits = 0u64;

    for rank in 0..8 {
        bits |= 1u64 << (rank * 8 + file.to_index());
    }

    BitBoard(bits)
}

#[inline]
fn king_zone(king: Square) -> BitBoard {
    let file = king.get_file().to_index() as i32;
    let rank = king.get_rank().to_index() as i32;

    let mut bits = 0u64;

    for df in -1..=1 {
        for dr in -1..=1 {
            let f = file + df;
            let r = rank + dr;

            if (0..8).contains(&f) && (0..8).contains(&r) {
                bits |= 1u64 << (r * 8 + f);
            }
        }
    }

    BitBoard(bits)
}

#[inline]
fn king_ring(king: Square) -> BitBoard {
    let file = king.get_file().to_index() as i32;
    let rank = king.get_rank().to_index() as i32;

    let mut bits = 0u64;

    for df in -1..=1 {
        for dr in -1..=1 {
            if df == 0 && dr == 0 {
                continue;
            }

            let f = file + df;
            let r = rank + dr;

            if (0..8).contains(&f) && (0..8).contains(&r) {
                bits |= 1u64 << (r * 8 + f);
            }
        }
    }

    BitBoard(bits)
}

#[inline]
fn pawn_shield_score(
    board: &Board,
    color: Color,
    king: Square,
) -> i32 {
    let pawns =
        *board.color_combined(color)
            & *board.pieces(Piece::Pawn);

    let king_file =
        king.get_file().to_index() as i32;

    let shield_rank =
        match color {
            Color::White => 1,
            Color::Black => 6,
        };

    let mut score = 0;

    for df in -1..=1 {
        let file = king_file + df;

        if !(0..8).contains(&file) {
            continue;
        }

        let square =
            Square::make_square(
                chess::Rank::from_index(shield_rank),
                File::from_index(file as usize),
            );

        let occupied =
            pawns & BitBoard::from_square(square);

        if occupied.popcnt() > 0 {
            score += PAWN_SHIELD_BONUS;
        } else {
            score -= MISSING_SHIELD_PENALTY;
        }
    }

    score
}

#[inline]
fn king_file_pressure(
    board: &Board,
    color: Color,
    king: Square,
) -> i32 {
    let enemy = !color;

    let friendly_pawns =
        *board.color_combined(color)
            & *board.pieces(Piece::Pawn);

    let enemy_pawns =
        *board.color_combined(enemy)
            & *board.pieces(Piece::Pawn);

    let file = king.get_file();

    let mask = file_mask(file);

    let own_pawn = (friendly_pawns & mask).popcnt();
    let enemy_pawn = (enemy_pawns & mask).popcnt();

    let mut score = 0;

    if own_pawn == 0 {
        score -= OPEN_FILE_PENALTY;
    }

    if enemy_pawn == 0 {
        score -= SEMI_OPEN_FILE_PENALTY;
    }

    score
}

#[inline]
fn king_attack_pressure(board: &Board, color: Color, king: Square) -> i32 {
    let enemy = !color;

    let zone = king_zone(king);
    let ring = king_ring(king);

    let enemy_pawns = *board.color_combined(enemy) & *board.pieces(Piece::Pawn);
    let enemy_knights = *board.color_combined(enemy) & *board.pieces(Piece::Knight);
    let enemy_bishops = *board.color_combined(enemy) & *board.pieces(Piece::Bishop);
    let enemy_rooks = *board.color_combined(enemy) & *board.pieces(Piece::Rook);
    let enemy_queens = *board.color_combined(enemy) & *board.pieces(Piece::Queen);

    let mut pressure = 0;

    // Pawn attacks into the king ring.
    for sq in enemy_pawns {
        let attacks =
            pawn_attacks(sq, enemy);

        pressure +=
            (attacks & ring).popcnt() as i32
                * KING_ATTACK_BONUS;
    }

    // Knight attacks into the king ring.
    for sq in enemy_knights {
        let attacks = chess::get_knight_moves(sq);

        pressure +=
            (attacks & ring).popcnt() as i32
                * KING_ATTACK_BONUS;
    }

    // Bishop attacks.
    let occupied = *board.combined();

    for sq in enemy_bishops {
        let attacks =
            chess::get_bishop_moves(
                sq,
                occupied,
            );

        pressure +=
            (attacks & zone).popcnt() as i32
                * KING_ZONE_ATTACK_BONUS;
    }

    // Rook attacks.
    for sq in enemy_rooks {
        let attacks =
            chess::get_rook_moves(
                sq,
                occupied,
            );

        pressure +=
            (attacks & zone).popcnt() as i32
                * KING_ZONE_ATTACK_BONUS;
    }

    // Queen attacks.
    for sq in enemy_queens {
        let attacks =
            chess::get_bishop_moves(
                sq,
                occupied,
            )
                | chess::get_rook_moves(
                sq,
                occupied,
            );

        pressure +=
            (attacks & zone).popcnt() as i32
                * KING_ZONE_ATTACK_BONUS;
    }

    pressure
}

#[inline]
fn pawn_attacks(
    square: Square,
    color: Color,
) -> BitBoard {
    let file = square.get_file().to_index() as i32;
    let rank = square.get_rank().to_index() as i32;

    let mut bits = 0u64;

    let direction =
        match color {
            Color::White => 1,
            Color::Black => -1,
        };

    let target_rank =
        rank + direction;

    if !(0..8).contains(&target_rank) {
        return BitBoard(0);
    }

    if file > 0 {
        bits |=
            1u64
                << (target_rank * 8 + file - 1);
    }

    if file < 7 {
        bits |=
            1u64
                << (target_rank * 8 + file + 1);
    }

    BitBoard(bits)
}

#[inline]
fn central_king_penalty(
    king: Square,
    color: Color,
) -> i32 {
    let file =
        king.get_file().to_index();

    let rank =
        king.get_rank().to_index();

    let home_rank =
        match color {
            Color::White => 0,
            Color::Black => 7,
        };

    if rank != home_rank {
        return CENTER_KING_PENALTY;
    }

    if file >= 2 && file <= 5 {
        return CENTER_KING_PENALTY;
    }

    0
}

#[inline]
fn king_safety_for(board: &Board, color: Color) -> (i32, i32) {
    let king = board.king_square(color);

    let mut mg = 0;

    mg += pawn_shield_score(board, color, king);
    mg += king_file_pressure(board, color, king);
    mg -= king_attack_pressure(board, color, king);

    let castled = match color {
        Color::White => king == Square::G1 || king == Square::C1,
        Color::Black => king == Square::G8 || king == Square::C8,
    };

    if castled {
        mg += CASTLED_KING_BONUS;
    }


    mg -= central_king_penalty(
        king,
        color,
    );

    (mg, mg / 4)
}

pub(crate) fn king_safety_score(board: &Board) -> (i32, i32) {
    let (white_mg, white_eg) = king_safety_for(board, Color::White);
    let (black_mg, black_eg) = king_safety_for(board, Color::Black);

    (white_mg - black_mg, white_eg - black_eg)
}