use chess::Board;
use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::{Duration, Instant};

use rchessengine::search::{GameHistory, Search};

const BENCH_DEPTH: u32 = 10;
const BENCH_THREADS: usize = 16;

struct BenchPosition {
    name: &'static str,
    fen: &'static str,
}

const POSITIONS: &[BenchPosition] = &[
    BenchPosition {
        name: "startpos",
        fen: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
    },
    BenchPosition {
        name: "tactical_1",
        fen: "r1bq1rk1/ppp2ppp/2n5/3pP3/1b1P4/2N5/PPP1NPPP/R1BQKB1R w KQ - 0 1",
    },
    BenchPosition {
        name: "tactical_2",
        fen: "r2q1rk1/ppp1bppp/2n1pn2/8/2B1P3/2N1B3/PPP2PPP/R2QK2R w KQ - 0 1",
    },
    BenchPosition {
        name: "middlegame_1",
        fen: "r1bq1rk1/pp2bppp/2n1pn2/2p5/3pP3/1P1P1N2/PBP1NPPP/R1BQ1RK1 w - - 0 1",
    },
    BenchPosition {
        name: "middlegame_2",
        fen: "r3r1k1/ppp2ppp/2n1bn2/8/2B1P3/2N1B3/PPP2PPP/R2Q1RK1 w - - 0 1",
    },
    BenchPosition {
        name: "endgame_1",
        fen: "8/5pk1/4p2p/1p2P2P/1P3PP1/6K1/8/8 w - - 0 1",
    },
    BenchPosition {
        name: "endgame_2",
        fen: "8/5pk1/4p3/1p2P2p/1P3P2/5K2/8/8 w - - 0 1",
    },
    BenchPosition {
        name: "queen_endgame",
        fen: "8/5pk1/4p3/3qP3/1P3P2/5P2/5K2/3Q4 w - - 0 1",
    },
];

fn next_benchmark_path() -> PathBuf {
    let version = env!("CARGO_PKG_VERSION");

    let directory = PathBuf::from("benchmarks");

    fs::create_dir_all(&directory)
        .expect("failed to create benchmarks directory");

    let first = directory.join(format!("{version}.txt"));

    if !first.exists() {
        return first;
    }

    let mut n = 2;

    loop {
        let path = directory.join(format!("{version}-v{n}.txt"));

        if !path.exists() {
            return path;
        }

        n += 1;
    }
}

fn run_benchmark() -> String {
    let depth = BENCH_DEPTH;

    let mut output = String::new();

    output.push_str("rchessengine benchmark\n");
    output.push_str(&format!(
        "version: {}\n",
        env!("CARGO_PKG_VERSION")
    ));
    output.push_str(&format!("depth: {}\n", depth));
    output.push_str(&format!(
        "threads: {}\n",
        BENCH_THREADS
    ));
    output.push_str(&format!(
        "positions: {}\n\n",
        POSITIONS.len()
    ));

    let suite_start = Instant::now();

    let mut total_nodes = 0u64;
    let mut total_tt_hits = 0u64;
    let mut total_tt_cutoffs = 0u64;

    for position in POSITIONS {
        let board = Board::from_str(position.fen)
            .unwrap_or_else(|error| {
                panic!(
                    "invalid FEN for {}: {}",
                    position.name,
                    error
                )
            });

        // Keep each position isolated so TT contents from one
        // position cannot affect another position's result.
        let mut search = Search::new();
        search.set_threads(BENCH_THREADS);

        let mut history = GameHistory::new();

        history.push(board.get_hash());

        let start = Instant::now();

        let result = search.search_timed(
            &board,
            depth,
            &mut history,
            Duration::from_secs(3600),
        );

        let elapsed = start.elapsed();
        let elapsed_seconds = elapsed.as_secs_f64();

        let nps = if elapsed_seconds > 0.0 {
            result.nodes as f64 / elapsed_seconds
        } else {
            0.0
        };

        total_nodes += result.nodes;
        total_tt_hits += result.tt_hits;
        total_tt_cutoffs += result.tt_cutoffs;

        let bestmove = result
            .best_move
            .map(|mv| mv.to_string())
            .unwrap_or_else(|| "0000".to_string());

        output.push_str(&format!(
            "{:<18} depth {:>2}  nodes {:>12}  nps {:>10.0}  \
             tt_hits {:>9}  tt_cutoffs {:>9}  score {:>10}  bestmove {}\n",
            position.name,
            result.depth_reached,
            result.nodes,
            nps,
            result.tt_hits,
            result.tt_cutoffs,
            result.score,
            bestmove,
        ));
    }

    let elapsed = suite_start.elapsed();
    let elapsed_seconds = elapsed.as_secs_f64();

    let total_nps = if elapsed_seconds > 0.0 {
        total_nodes as f64 / elapsed_seconds
    } else {
        0.0
    };

    let tt_hit_rate = if total_nodes > 0 {
        total_tt_hits as f64 / total_nodes as f64 * 100.0
    } else {
        0.0
    };

    let tt_cutoff_rate = if total_tt_hits > 0 {
        total_tt_cutoffs as f64 / total_tt_hits as f64 * 100.0
    } else {
        0.0
    };

    output.push('\n');
    output.push_str("==============================\n");
    output.push_str("BENCH COMPLETE\n");
    output.push_str("==============================\n");

    output.push_str(&format!(
        "version:      {}\n",
        env!("CARGO_PKG_VERSION")
    ));

    output.push_str(&format!(
        "depth:        {}\n",
        BENCH_DEPTH
    ));

    output.push_str(&format!(
        "threads:      {}\n",
        BENCH_THREADS
    ));

    output.push_str(&format!(
        "positions:    {}\n",
        POSITIONS.len()
    ));
    output.push_str(&format!(
        "total nodes:  {}\n",
        total_nodes
    ));
    output.push_str(&format!(
        "total time:   {:.3}s\n",
        elapsed_seconds
    ));
    output.push_str(&format!(
        "total NPS:    {:.0}\n",
        total_nps
    ));
    output.push_str(&format!(
        "TT hits:      {}\n",
        total_tt_hits
    ));
    output.push_str(&format!(
        "TT cutoffs:   {}\n",
        total_tt_cutoffs
    ));
    output.push_str(&format!(
        "TT hit rate:  {:.2}%\n",
        tt_hit_rate
    ));
    output.push_str(&format!(
        "TT cutoff:    {:.2}% of hits\n",
        tt_cutoff_rate
    ));

    output
}

fn main() {
    let output = run_benchmark();

    print!("{output}");

    let path = next_benchmark_path();

    let mut file = File::create(&path)
        .expect("failed to create benchmark result file");

    writeln!(
        file,
        "rchessengine benchmark snapshot"
    )
        .unwrap();

    writeln!(
        file,
        "version: {}",
        env!("CARGO_PKG_VERSION")
    )
        .unwrap();

    writeln!(
        file,
        "command: cargo bench"
    )
        .unwrap();

    writeln!(
        file,
        "depth: {}",
        BENCH_DEPTH
    )
        .unwrap();

    writeln!(
        file,
        "threads: {}",
        BENCH_THREADS
    )
        .unwrap();

    writeln!(file).unwrap();

    file.write_all(output.as_bytes())
        .expect("failed to write benchmark result");

    println!();
    println!(
        "Benchmark saved to {}",
        path.display()
    );
}