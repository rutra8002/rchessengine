use chess::{
    get_bishop_moves, get_king_moves, get_knight_moves, get_rook_moves, get_pawn_moves, Board, BoardStatus,
    ChessMove, Color, MoveGen, Piece, EMPTY,
};

pub const DEFAULT_DEPTH: u32 = 7;
const INF: i32 = i32::MAX / 2;
const MATE_SCORE: i32 = 900_000;

const MOBILITY_WEIGHT: i32 = 2;

// TODO: split code into multiple files

const TT_BITS: usize = 20;
const TT_SIZE: usize = 1 << TT_BITS;
const TT_MASK: usize = TT_SIZE - 1;

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum Bound {
    Exact = 0,
    Lower = 1,
    Upper = 2,
}

#[derive(Clone, Copy)]
struct TTEntry {
    key: u64,
    depth: i16,
    score: i32,
    bound: Bound,
    best_move: Option<ChessMove>,
}

impl Default for TTEntry {
    fn default() -> Self {
        Self {
            key: 0,
            depth: -1,
            score: 0,
            bound: Bound::Exact,
            best_move: None,
        }
    }
}

struct TranspositionTable {
    entries: Vec<TTEntry>,
}

impl TranspositionTable {
    fn new() -> Self {
        Self {
            entries: vec![TTEntry::default(); TT_SIZE],
        }
    }

    #[inline]
    fn index(key: u64) -> usize {
        (key as usize) & TT_MASK
    }

    #[inline]
    fn probe(&self, key: u64) -> Option<&TTEntry> {
        let entry = &self.entries[Self::index(key)];

        if entry.depth >= 0 && entry.key == key {
            Some(entry)
        } else {
            None
        }
    }

    #[inline]
    fn store(
        &mut self,
        key: u64,
        depth: u32,
        score: i32,
        bound: Bound,
        best_move: Option<ChessMove>,
    ) {
        let index = Self::index(key);
        let old = self.entries[index];

        if old.depth < depth as i16 || old.key != key {
            self.entries[index] = TTEntry {
                key,
                depth: depth as i16,
                score,
                bound,
                best_move,
            };
        }
    }

    fn clear(&mut self) {
        self.entries.fill(TTEntry::default());
    }
}

pub struct SearchStats {
    pub nodes: u64,
    pub tt_hits: u64,
    pub tt_cutoffs: u64,
}

pub struct Search {
    tt: TranspositionTable,
}

impl Search {
    pub fn new() -> Self {
        Self {
            tt: TranspositionTable::new(),
        }
    }

    pub fn clear_tt(&mut self) {
        self.tt.clear();
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
                Piece::Queen => {
                    get_bishop_moves(sq, occupied) | get_rook_moves(sq, occupied)
                }
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

    #[inline]
    fn evaluate(board: &Board) -> i32 {
        let mut score = 0;

        for sq in *board.combined() {
            if let Some(piece) = board.piece_on(sq) {
                let value = Self::piece_value(piece);
                let color = board.color_on(sq).unwrap();

                score += if color == Color::White {
                    value
                } else {
                    -value
                };
            }
        }

        score += MOBILITY_WEIGHT * Self::mobility_score(board);

        score
    }

    #[inline]
    fn evaluate_relative(board: &Board) -> i32 {
        let score = Self::evaluate(board);

        if board.side_to_move() == Color::White {
            score
        } else {
            -score
        }
    }

    #[inline]
    fn move_order_score(board: &Board, m: ChessMove) -> i32 {
        if let Some(victim) = board.piece_on(m.get_dest()) {
            let attacker_value = board
                .piece_on(m.get_source())
                .map(Self::piece_value)
                .unwrap_or(0);

            10_000 + Self::piece_value(victim) * 10 - attacker_value
        } else {
            0
        }
    }

    fn ordered_legal_moves(
        board: &Board,
        tt_move: Option<ChessMove>,
    ) -> Vec<ChessMove> {
        let mut moves: Vec<ChessMove> = MoveGen::new_legal(board).collect();

        moves.sort_unstable_by_key(|&m| {
            let tt_bonus = if Some(m) == tt_move {
                1_000_000
            } else {
                0
            };

            -(tt_bonus + Self::move_order_score(board, m))
        });

        moves
    }

    fn quiescence(
        &mut self,
        board: &Board,
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

        let stand_pat = Self::evaluate_relative(board);

        if stand_pat >= beta {
            return beta;
        }

        if stand_pat > alpha {
            alpha = stand_pat;
        }

        let targets = *board.color_combined(!board.side_to_move());

        let mut movegen = MoveGen::new_legal(board);
        movegen.set_iterator_mask(targets);

        let mut captures: Vec<ChessMove> = movegen.collect();

        captures.sort_unstable_by_key(|&m| {
            -Self::move_order_score(board, m)
        });

        for m in captures {
            let next = board.make_move_new(m);

            let score = -self.quiescence(
                &next,
                -beta,
                -alpha,
                ply + 1,
                stats,
            );

            if score >= beta {
                return beta;
            }

            if score > alpha {
                alpha = score;
            }
        }

        alpha
    }

    fn negamax(
        &mut self,
        board: &Board,
        depth: u32,
        mut alpha: i32,
        beta: i32,
        ply: i32,
        stats: &mut SearchStats,
        history: &mut Vec<u64>,
    ) -> i32 {
        stats.nodes += 1;

        let hash = board.get_hash();

        // Repetition is path-dependent, so check it before TT probing.
        let repetitions = history.iter().filter(|&&h| h == hash).count();

        if repetitions >= 2 {
            return 0;
        }

        match board.status() {
            BoardStatus::Checkmate => return -MATE_SCORE + ply,
            BoardStatus::Stalemate => return 0,
            BoardStatus::Ongoing => {}
        }

        if depth == 0 {
            return self.quiescence(
                board,
                alpha,
                beta,
                ply,
                stats,
            );
        }

        let original_alpha = alpha;

        let tt_move = if let Some(entry) = self.tt.probe(hash) {
            if entry.depth >= depth as i16 {
                stats.tt_hits += 1;

                match entry.bound {
                    Bound::Exact => {
                        stats.tt_cutoffs += 1;
                        return entry.score;
                    }

                    Bound::Lower if entry.score >= beta => {
                        stats.tt_cutoffs += 1;
                        return entry.score;
                    }

                    Bound::Upper if entry.score <= alpha => {
                        stats.tt_cutoffs += 1;
                        return entry.score;
                    }

                    _ => {}
                }
            }

            entry.best_move
        } else {
            None
        };

        // If this exact position/depth was found, return it.
        if let Some(entry) = self.tt.probe(hash) {
            if entry.depth >= depth as i16 {
                match entry.bound {
                    Bound::Exact => {
                        return entry.score;
                    }

                    Bound::Lower if entry.score >= beta => {
                        return entry.score;
                    }

                    Bound::Upper if entry.score <= alpha => {
                        return entry.score;
                    }

                    _ => {}
                }
            }
        }

        let moves = Self::ordered_legal_moves(board, tt_move);

        if moves.is_empty() {
            return 0;
        }

        let mut best = -INF;
        let mut best_move = None;

        for m in moves {
            let next = board.make_move_new(m);

            history.push(next.get_hash());

            let score = -self.negamax(
                &next,
                depth - 1,
                -beta,
                -alpha,
                ply + 1,
                stats,
                history,
            );

            history.pop();

            if score > best {
                best = score;
                best_move = Some(m);
            }

            if score > alpha {
                alpha = score;
            }

            if alpha >= beta {
                break;
            }
        }

        let bound = if best <= original_alpha {
            Bound::Upper
        } else if best >= beta {
            Bound::Lower
        } else {
            Bound::Exact
        };

        self.tt.store(
            hash,
            depth,
            best,
            bound,
            best_move,
        );

        best
    }

    pub fn search_best_move(
        &mut self,
        board: &Board,
        depth: u32,
        history: &mut Vec<u64>,
    ) -> (Option<ChessMove>, i32, u64, u64, u64) {
        let mut stats = SearchStats {
            nodes: 0,
            tt_hits: 0,
            tt_cutoffs: 0,
        };

        let mut alpha = -INF;
        let beta = INF;

        let mut best_move = None;
        let mut best_score = -INF;

        let hash = board.get_hash();

        let tt_move = self.tt.probe(hash).and_then(|e| e.best_move);

        let moves = Self::ordered_legal_moves(board, tt_move);

        for m in moves {
            let next = board.make_move_new(m);

            history.push(next.get_hash());

            let score = -self.negamax(
                &next,
                depth.saturating_sub(1),
                -beta,
                -alpha,
                1,
                &mut stats,
                history,
            );

            history.pop();

            if score > best_score || best_move.is_none() {
                best_score = score;
                best_move = Some(m);
            }

            if best_score > alpha {
                alpha = best_score;
            }
        }

        self.tt.store(
            hash,
            depth,
            best_score,
            Bound::Exact,
            best_move,
        );

        (
            best_move,
            best_score,
            stats.nodes,
            stats.tt_hits,
            stats.tt_cutoffs,
        )
    }
}