mod negamax;
mod quiescence;
mod transposition;
mod history;
mod heuristics;

use std::sync::Arc;
use std::time::Duration;

use chess::{Board, ChessMove};

use crate::ordering::ordered_legal_moves;
use crate::time::SearchDeadline;

use transposition::{Bound, TranspositionTable};

use heuristics::HistoryHeuristic;

pub use history::GameHistory;

pub const MAX_DEPTH: u32 = 64;

const MAX_PLY: usize = MAX_DEPTH as usize + 8;

const INF: i32 = i32::MAX / 2;

pub(crate) const MATE_SCORE: i32 = 67_000_000;

const MATE_THRESHOLD: i32 = MATE_SCORE - 1000;


pub fn format_uci_score(score: i32) -> String {
    if score.abs() >= MATE_THRESHOLD {
        let plies_to_mate = MATE_SCORE - score.abs();

        let moves_to_mate = (plies_to_mate + 1) / 2;

        if score > 0 {
            format!("mate {}", moves_to_mate)
        } else {
            format!("mate -{}", moves_to_mate)
        }
    } else {
        format!("cp {}", score)
    }
}

const TIME_CHECK_INTERVAL: u64 = 2048;

pub struct SearchStats {
    pub nodes: u64,
    pub tt_hits: u64,
    pub tt_cutoffs: u64,
    pub(crate) deadline: Option<SearchDeadline>,
    pub(crate) stopped: bool,
}

impl SearchStats {
    fn new(deadline: Option<SearchDeadline>) -> Self {
        Self {
            nodes: 0,
            tt_hits: 0,
            tt_cutoffs: 0,
            deadline,
            stopped: false,
        }
    }

    #[inline]
    pub(crate) fn check_time(&mut self) {
        if self.stopped {
            return;
        }

        if !self.nodes.is_multiple_of(TIME_CHECK_INTERVAL) {
            return;
        }

        if let Some(deadline) = self.deadline {
            if deadline.expired() {
                self.stopped = true;
            }
        }
    }
}

pub struct Search {
    tt: Arc<TranspositionTable>,
    pub(crate) killers: Vec<[Option<ChessMove>; 2]>,
    pub(crate) history_table: HistoryHeuristic,
    threads: usize,
}

pub struct SearchResult {
    pub best_move: Option<ChessMove>,
    pub score: i32,
    pub depth_reached: u32,
    pub nodes: u64,
    pub tt_hits: u64,
    pub tt_cutoffs: u64,
}

impl Search {
    pub fn new() -> Self {
        Self {
            tt: Arc::new(TranspositionTable::new()),
            killers: vec![[None, None]; MAX_PLY],
            history_table: HistoryHeuristic::new(),
            threads: 1,
        }
    }

    fn spawn_worker(tt: Arc<TranspositionTable>) -> Self {
        Self {
            tt,
            killers: vec![[None, None]; MAX_PLY],
            history_table: HistoryHeuristic::new(),
            threads: 1,
        }
    }

    pub fn set_threads(&mut self, threads: usize) {
        self.threads = threads.max(1);
    }
    
    pub fn clear_tt(&mut self) {
        self.tt.clear();
        self.history_table.clear();
        self.killers.iter_mut().for_each(|k| *k = [None, None]);
    }

    pub fn search_best_move(
        &mut self,
        board: &Board,
        depth: u32,
        history: &mut GameHistory,
    ) -> (Option<ChessMove>, i32, u64, u64, u64) {
        let result = self.search_iterative_smp(board, depth, history, None);

        (
            result.best_move,
            result.score,
            result.nodes,
            result.tt_hits,
            result.tt_cutoffs,
        )
    }

    pub fn search_timed(
        &mut self,
        board: &Board,
        max_depth: u32,
        history: &mut GameHistory,
        time_budget: Duration,
    ) -> SearchResult {
        let deadline = SearchDeadline::new(time_budget);

        self.search_iterative_smp(board, max_depth, history, Some(deadline))
    }

    fn search_iterative_smp(
        &mut self,
        board: &Board,
        max_depth: u32,
        history: &mut GameHistory,
        deadline: Option<SearchDeadline>,
    ) -> SearchResult {
        let helper_count = self.threads.saturating_sub(1);

        if helper_count == 0 {
            return self.search_iterative(board, max_depth, history, deadline, 0);
        }

        let mut helpers: Vec<Search> = (0..helper_count)
            .map(|_| Search::spawn_worker(Arc::clone(&self.tt)))
            .collect();

        let mut worker_histories: Vec<GameHistory> =
            (0..helper_count).map(|_| history.clone()).collect();

        std::thread::scope(|scope| {
            let handles: Vec<_> = helpers
                .iter_mut()
                .zip(worker_histories.iter_mut())
                .enumerate()
                .map(|(i, (worker, worker_history))| {
                    let depth_bump = if deadline.is_some() {
                        ((i + 1) % 2) as u32
                    } else {
                        0
                    };
                    let worker_depth = (max_depth + depth_bump).min(MAX_DEPTH);

                    scope.spawn(move || {
                        worker.search_iterative(
                            board,
                            worker_depth,
                            worker_history,
                            deadline,
                            i + 1,
                        )
                    })
                })
                .collect();

            let mut best = self.search_iterative(board, max_depth, history, deadline, 0);

            for handle in handles {
                if let Ok(result) = handle.join() {
                    best.nodes += result.nodes;
                    best.tt_hits += result.tt_hits;
                    best.tt_cutoffs += result.tt_cutoffs;

                    if result.best_move.is_some() && result.depth_reached > best.depth_reached {
                        best.best_move = result.best_move;
                        best.score = result.score;
                        best.depth_reached = result.depth_reached;
                    }
                }
            }

            best
        })
    }

    fn search_iterative(
        &mut self,
        board: &Board,
        max_depth: u32,
        history: &mut GameHistory,
        deadline: Option<SearchDeadline>,
        worker_id: usize,
    ) -> SearchResult {
        self.killers
            .iter_mut()
            .for_each(|k| *k = [None, None]);

        self.history_table.clear();

        let mut total_nodes = 0u64;
        let mut total_tt_hits = 0u64;
        let mut total_tt_cutoffs = 0u64;

        let mut best_move = None;
        let mut best_score = 0;
        let mut depth_reached = 0;

        for depth in 1..=max_depth.max(1) {
            if let Some(deadline) = deadline {
                if depth > 1
                    && deadline.remaining()
                    < Duration::from_millis(50)
                {
                    break;
                }
            }

            let mut stats = SearchStats::new(deadline);

            let (root_move, root_score) = self.search_root(
                board,
                depth,
                history,
                &mut stats,
            );

            total_nodes += stats.nodes;
            total_tt_hits += stats.tt_hits;
            total_tt_cutoffs += stats.tt_cutoffs;

            if stats.stopped {
                break;
            }

            if let Some(m) = root_move {
                best_move = Some(m);
                best_score = root_score;
                depth_reached = depth;
            }

            if root_score > MATE_SCORE - 10000 {
                break;
            }

            if worker_id == 0 {
                eprintln!(
                    "info depth {} score {} nodes {}",
                    depth,
                    format_uci_score(root_score),
                    total_nodes
                );
            }
        }

        SearchResult {
            best_move,
            score: best_score,
            depth_reached,
            nodes: total_nodes,
            tt_hits: total_tt_hits,
            tt_cutoffs: total_tt_cutoffs,
        }
    }

    fn search_root(
        &mut self,
        board: &Board,
        depth: u32,
        history: &mut GameHistory,
        stats: &mut SearchStats,
    ) -> (Option<ChessMove>, i32) {
        let mut alpha = -INF;
        let beta = INF;

        let mut best_move = None;
        let mut best_score = -INF;

        let hash = board.get_hash();

        let tt_move = self
            .tt
            .probe(hash)
            .and_then(|entry| entry.best_move);

        let side_to_move = board.side_to_move();
        let killers_here = self.killers[0];

        let moves = {
            let history_table = &self.history_table;

            ordered_legal_moves(board, tt_move, killers_here, |m| {
                history_table.score(side_to_move, m)
            })
        };

        for (move_index, m) in moves.into_iter().enumerate() {
            let next = board.make_move_new(m);

            history.push(next.get_hash());

            let score = if move_index == 0 {
                -negamax::negamax(
                    self,
                    &next,
                    depth.saturating_sub(1),
                    -beta,
                    -alpha,
                    1,
                    stats,
                    history,
                )
            } else {
                let mut score = -negamax::negamax(
                    self,
                    &next,
                    depth.saturating_sub(1),
                    -alpha - 1,
                    -alpha,
                    1,
                    stats,
                    history,
                );

                if score > alpha && score < beta {
                    score = -negamax::negamax(
                        self,
                        &next,
                        depth.saturating_sub(1),
                        -beta,
                        -alpha,
                        1,
                        stats,
                        history,
                    );
                }

                score
            };

            history.pop();

            if stats.stopped {
                break;
            }

            if score > best_score || best_move.is_none() {
                best_score = score;
                best_move = Some(m);
            }

            if best_score > alpha {
                alpha = best_score;
            }

            if alpha >= beta {
                break;
            }
        }

        if !stats.stopped {
            self.tt.store(
                hash,
                depth,
                transposition::score_to_tt(best_score, 0),
                Bound::Exact,
                best_move,
            );
        }

        (best_move, best_score)
    }
}