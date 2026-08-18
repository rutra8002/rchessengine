use chess::{
    get_bishop_moves, get_king_moves, get_knight_moves, get_rook_moves, get_pawn_moves, Board, BoardStatus,
    ChessMove, Color, MoveGen, Piece, EMPTY,
};

pub const DEFAULT_DEPTH: u32 = 6;
const INF: i32 = i32::MAX / 2;
const MATE_SCORE: i32 = 900_000;

const MOBILITY_WEIGHT: i32 = 2;

pub struct SearchStats {
    pub nodes: u64,
}

fn piece_value(p: Piece) -> i32 {
    match p {
        Piece::Pawn => 100,
        Piece::Knight => 300,
        Piece::Bishop => 330,
        Piece::Rook => 500,
        Piece::Queen => 900,
        Piece::King => 0,
    }
}

fn mobility_score(board: &Board) -> i32 {
    let occupied = *board.combined();
    let white_pieces = *board.color_combined(Color::White);
    let black_pieces = *board.color_combined(Color::Black);

    let mut white_mobility = 0i32;
    let mut black_mobility = 0i32;

    for sq in occupied {
        let piece = match board.piece_on(sq) {
            Some(p) => p,
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

    white_mobility - black_mobility
}

fn evaluate(board: &Board) -> i32 {
    let mut score = 0;
    for sq in *board.combined() {
        if let Some(piece) = board.piece_on(sq) {
            let v = piece_value(piece);
            let color = board.color_on(sq).unwrap();
            score += if color == Color::White { v } else { -v };
        }
    }
    score += MOBILITY_WEIGHT * mobility_score(board);
    score
}

fn negamax(
    board: &Board,
    depth: u32,
    mut alpha: i32,
    beta: i32,
    ply: i32,
    stats: &mut SearchStats,
    history: &mut Vec<u64>,
) -> i32 {
    stats.nodes += 1;

    // repetitią
    let current_hash = board.get_hash();
    let repetitions = history.iter().filter(|&&h| h == current_hash).count();
    if repetitions >= 2 {
        return 0; 
    }

    match board.status() {
        BoardStatus::Checkmate => return -MATE_SCORE + ply,
        BoardStatus::Stalemate => return 0,
        BoardStatus::Ongoing => {}
    }

    if depth == 0 {
        let e = evaluate(board);
        return if board.side_to_move() == Color::White { e } else { -e };
    }

    let mut best = -INF;
    let moves = MoveGen::new_legal(board);
    for m in moves {
        let next = board.make_move_new(m);

        history.push(next.get_hash());
        let score = -negamax(&next, depth - 1, -beta, -alpha, ply + 1, stats, history);
        history.pop();

        if score > best {
            best = score;
        }
        if best > alpha {
            alpha = best;
        }
        if alpha >= beta {
            break;
        }
    }
    best
}

pub fn search_best_move(
    board: &Board,
    depth: u32,
    history: &mut Vec<u64>
) -> (Option<ChessMove>, i32, u64) {
    let mut stats = SearchStats { nodes: 0 };
    let mut alpha = -INF;
    let beta = INF;

    let mut best_move: Option<ChessMove> = None;
    let mut best_score = -INF;

    for m in MoveGen::new_legal(board) {
        let next = board.make_move_new(m);

        history.push(next.get_hash());
        let score = -negamax(&next, depth.saturating_sub(1), -beta, -alpha, 1, &mut stats, history);
        history.pop();

        if score > best_score || best_move.is_none() {
            best_score = score;
            best_move = Some(m);
        }
        if best_score > alpha {
            alpha = best_score;
        }
    }

    (best_move, best_score, stats.nodes)
}