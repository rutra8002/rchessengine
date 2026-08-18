use chess::{Board, BoardStatus, ChessMove, Color, MoveGen, Piece};

pub const DEFAULT_DEPTH: u32 = 6;
const INF: i32 = i32::MAX / 2;
const MATE_SCORE: i32 = 900_000;

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

fn evaluate(board: &Board) -> i32 {
    let mut score = 0;
    for sq in *board.combined() {
        if let Some(piece) = board.piece_on(sq) {
            let v = piece_value(piece);
            let color = board.color_on(sq).unwrap();
            score += if color == Color::White { v } else { -v };
        }
    }
    score
}

fn negamax(
    board: &Board,
    depth: u32,
    mut alpha: i32,
    beta: i32,
    ply: i32,
    stats: &mut SearchStats,
) -> i32 {
    stats.nodes += 1;

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
        let score = -negamax(&next, depth - 1, -beta, -alpha, ply + 1, stats);

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

pub fn search_best_move(board: &Board, depth: u32) -> (Option<ChessMove>, i32, u64) {
    let mut stats = SearchStats { nodes: 0 };
    let mut alpha = -INF;
    let beta = INF;

    let mut best_move: Option<ChessMove> = None;
    let mut best_score = -INF;

    for m in MoveGen::new_legal(board) {
        let next = board.make_move_new(m);
        let score = -negamax(&next, depth.saturating_sub(1), -beta, -alpha, 1, &mut stats);

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