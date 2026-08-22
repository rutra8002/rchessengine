mod negamax;
mod quiescence;
mod transposition;
mod history;
mod heuristics;

use std::sync::{
    Arc,
    mpsc::{self, Receiver, Sender},
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use chess::{Board, ChessMove};

use crate::ordering::ordered_legal_moves;
use crate::time::SearchDeadline;

use transposition::{Bound, TranspositionTable, DEFAULT_HASH_MB};

use heuristics::HistoryHeuristic;

pub use history::GameHistory;

pub const MAX_DEPTH: u32 = 64;

const MAX_PLY: usize = MAX_DEPTH as usize + 8;

const INF: i32 = i32::MAX / 2;

pub(crate) const MATE_SCORE: i32 = 67_000_000;

const MATE_THRESHOLD: i32 = MATE_SCORE - 1000;

const ASPIRATION_WINDOW: i32 = 25;

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

const TIME_CHECK_INTERVAL: u32 = 2048;

pub struct SearchStats {
    pub nodes: u64,
    pub tt_hits: u64,
    pub tt_cutoffs: u64,
    pub(crate) deadline: Option<SearchDeadline>,
    pub(crate) stopped: bool,
    time_check_counter: u32,
}

impl SearchStats {
    fn new(deadline: Option<SearchDeadline>) -> Self {
        Self {
            nodes: 0,
            tt_hits: 0,
            tt_cutoffs: 0,
            deadline,
            stopped: false,
            time_check_counter: TIME_CHECK_INTERVAL,
        }
    }

    #[inline]
    pub(crate) fn check_time(&mut self) {
        if self.stopped {
            return;
        }

        self.time_check_counter -= 1;

        if self.time_check_counter != 0 {
            return;
        }

        self.time_check_counter = TIME_CHECK_INTERVAL;

        if let Some(deadline) = self.deadline {
            if deadline.expired() {
                self.stopped = true;
            }
        }
    }
}

pub struct SearchResult {
    pub best_move: Option<ChessMove>,
    pub score: i32,
    pub depth_reached: u32,
    pub nodes: u64,
    pub tt_hits: u64,
    pub tt_cutoffs: u64,
}

pub struct Search {
    tt: Arc<TranspositionTable>,
    hash_mb: usize,
    pub(crate) killers: Vec<[Option<ChessMove>; 2]>,
    pub(crate) history_table: HistoryHeuristic,
    threads: usize,
    worker_pool: Option<WorkerPool>,
}

struct WorkerPool {
    workers: Vec<Worker>,
}

struct Worker {
    tx: Sender<WorkerCommand>,
    handle: Option<JoinHandle<()>>,
}

enum WorkerCommand {
    Search {
        board: Board,
        max_depth: u32,
        history: GameHistory,
        deadline: Option<SearchDeadline>,
        worker_id: usize,
        result_tx: Sender<SearchResult>,
    },

    Shutdown,
}

impl Search {
    pub fn new() -> Self {
        Self {
            tt: Arc::new(TranspositionTable::new()),
            hash_mb: DEFAULT_HASH_MB,
            killers: vec![[None, None]; MAX_PLY],
            history_table: HistoryHeuristic::new(),
            threads: 1,
            worker_pool: None,
        }
    }

    fn spawn_worker(tt: Arc<TranspositionTable>) -> Self {
        Self {
            tt,
            hash_mb: DEFAULT_HASH_MB,
            killers: vec![[None, None]; MAX_PLY],
            history_table: HistoryHeuristic::new(),
            threads: 1,
            worker_pool: None,
        }
    }

    pub fn set_threads(&mut self, threads: usize) {
        let threads = threads.max(1);

        if threads == self.threads {
            return;
        }

        self.shutdown_workers();

        self.threads = threads;

        if threads > 1 {
            self.worker_pool =
                Some(WorkerPool::new(threads - 1, Arc::clone(&self.tt)));
        }
    }

    pub fn set_hash_size_mb(&mut self, mb: usize) {
        let mb = mb.max(1);

        if mb == self.hash_mb {
            return;
        }

        self.shutdown_workers();

        self.tt = Arc::new(TranspositionTable::with_size_mb(mb));
        self.hash_mb = mb;

        self.killers.iter_mut().for_each(|k| *k = [None, None]);
        self.history_table.clear();

        if self.threads > 1 {
            self.worker_pool =
                Some(WorkerPool::new(self.threads - 1, Arc::clone(&self.tt)));
        }
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

        self.search_iterative_smp(
            board,
            max_depth,
            history,
            Some(deadline),
        )
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
            return self.search_iterative(
                board,
                max_depth,
                history,
                deadline,
                0,
            );
        }

        let Some(pool) = self.worker_pool.as_ref() else {
            return self.search_iterative(
                board,
                max_depth,
                history,
                deadline,
                0,
            );
        };

        let (result_tx, result_rx) = mpsc::channel();

        for (i, worker) in pool.workers.iter().enumerate() {
            let command = WorkerCommand::Search {
                board: board.clone(),
                max_depth,
                history: history.clone(),
                deadline,
                worker_id: i + 1,
                result_tx: result_tx.clone(),
            };

            let _ = worker.tx.send(command);
        }

        drop(result_tx);


        let mut best =
            self.search_iterative(board, max_depth, history, deadline, 0);


        for _ in 0..helper_count {
            match result_rx.recv() {
                Ok(result) => {
                    best.nodes += result.nodes;
                    best.tt_hits += result.tt_hits;
                    best.tt_cutoffs += result.tt_cutoffs;

                    if result.best_move.is_some()
                        && result.depth_reached > best.depth_reached
                    {
                        best.best_move = result.best_move;
                        best.score = result.score;
                        best.depth_reached = result.depth_reached;
                    }
                }

                Err(_) => break,
            }
        }

        best
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
                    && deadline.remaining() < Duration::from_millis(50)
                {
                    break;
                }
            }

            let mut stats = SearchStats::new(deadline);

            let aspiration = if depth > 1 && depth_reached > 0 {
                Some(best_score)
            } else {
                None
            };

            let (root_move, root_score) =
                self.search_root(board, depth, history, &mut stats, aspiration);

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
        aspiration: Option<i32>,
    ) -> (Option<ChessMove>, i32) {
        let mut window = ASPIRATION_WINDOW;

        let (mut alpha, mut beta) = match aspiration {
            Some(prev) if prev.abs() < MATE_THRESHOLD => {
                (prev.saturating_sub(window).max(-INF), prev.saturating_add(window).min(INF))
            }
            _ => (-INF, INF),
        };

        loop {
            let (best_move, best_score) =
                self.search_root_window(board, depth, history, stats, alpha, beta);

            if stats.stopped {
                return (best_move, best_score);
            }

            if best_score <= alpha && alpha > -INF {
                beta = ((alpha as i64 + beta as i64) / 2) as i32;
                window = window.saturating_mul(2);
                alpha = best_score.saturating_sub(window).max(-INF);
                continue;
            }

            if best_score >= beta && beta < INF {
                window = window.saturating_mul(2);
                beta = best_score.saturating_add(window).min(INF);
                continue;
            }

            return (best_move, best_score);
        }
    }

    fn search_root_window(
        &mut self,
        board: &Board,
        depth: u32,
        history: &mut GameHistory,
        stats: &mut SearchStats,
        alpha_init: i32,
        beta: i32,
    ) -> (Option<ChessMove>, i32) {
        let mut alpha = alpha_init;

        let mut best_move = None;
        let mut best_score = -INF;

        let hash = board.get_hash();

        let tt_move = self
            .tt
            .probe(hash)
            .and_then(|entry| entry.best_move());

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

    fn shutdown_workers(&mut self) {
        let Some(mut pool) = self.worker_pool.take() else {
            return;
        };


        for worker in &pool.workers {
            let _ = worker.tx.send(WorkerCommand::Shutdown);
        }


        for worker in &mut pool.workers {
            if let Some(handle) = worker.handle.take() {
                let _ = handle.join();
            }
        }
    }
}

impl Drop for Search {
    fn drop(&mut self) {
        self.shutdown_workers();
    }
}

impl WorkerPool {
    fn new(count: usize, tt: Arc<TranspositionTable>) -> Self {
        let mut workers = Vec::with_capacity(count);

        for worker_id in 0..count {
            let (tx, rx): (
                Sender<WorkerCommand>,
                Receiver<WorkerCommand>,
            ) = mpsc::channel();

            let worker_tt = Arc::clone(&tt);

            let handle = thread::spawn(move || {
                let mut search = Search::spawn_worker(worker_tt);

                while let Ok(command) = rx.recv() {
                    match command {
                        WorkerCommand::Search {
                            board,
                            max_depth,
                            mut history,
                            deadline,
                            worker_id: command_worker_id,
                            result_tx,
                        } => {
                            let result = search.search_iterative(
                                &board,
                                max_depth,
                                &mut history,
                                deadline,
                                command_worker_id,
                            );

                            let _ = result_tx.send(result);
                        }

                        WorkerCommand::Shutdown => {
                            break;
                        }
                    }
                }
            });

            let _ = worker_id;

            workers.push(Worker {
                tx,
                handle: Some(handle),
            });
        }

        Self { workers }
    }
}