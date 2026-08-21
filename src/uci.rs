use crate::search::{format_uci_score, GameHistory, Search, MAX_DEPTH};
use crate::time::{compute_time_budget, parse_go_params, phase_factor};

use chess::{Board, ChessMove, Piece, Square};
use std::io::{self, BufRead, Write};
use std::str::FromStr;

const ENGINE_NAME: &str = "rchessengine";
const ENGINE_AUTHOR: &str = "ruter";

fn handle_position(
    board: &mut Board,
    history: &mut GameHistory,
    tokens: &[&str],
) {
    let mut idx = 0;

    history.clear();

    if tokens.get(idx) == Some(&"startpos") {
        *board = Board::default();
        history.push(board.get_hash());
        idx += 1;
    } else if tokens.get(idx) == Some(&"fen") {
        idx += 1;
        let fen_fields: Vec<&str> = tokens[idx..].iter().take(6).cloned().collect();
        let fen = fen_fields.join(" ");
        idx += fen_fields.len();
        if let Ok(b) = Board::from_str(&fen) {
            *board = b;
            history.push(board.get_hash());
        }
    }

    if tokens.get(idx) == Some(&"moves") {
        idx += 1;
        for mv_str in &tokens[idx..] {
            match parse_uci_move(board, mv_str) {
                Some(m) if board.legal(m) => {
                    *board = board.make_move_new(m);
                    history.push(board.get_hash());
                }
                _ => break, // illegal move from the GUI
            }
        }
    }
}

fn parse_uci_move(_board: &Board, s: &str) -> Option<ChessMove> {
    if s.len() < 4 {
        return None;
    }
    let source = Square::from_str(&s[0..2]).ok()?;
    let dest = Square::from_str(&s[2..4]).ok()?;
    let promotion = if s.len() >= 5 {
        match &s[4..5] {
            "q" => Some(Piece::Queen),
            "r" => Some(Piece::Rook),
            "b" => Some(Piece::Bishop),
            "n" => Some(Piece::Knight),
            _ => None,
        }
    } else {
        None
    };
    Some(ChessMove::new(source, dest, promotion))
}

fn handle_go(
    board: &Board,
    history: &mut GameHistory,
    tokens: &[&str],
    search: &mut Search,
) {
    let params = parse_go_params(tokens);

    if let Some(depth) = params.depth {
        if params.wtime_ms.is_none()
            && params.btime_ms.is_none()
            && params.movetime_ms.is_none()
        {
            let (best, score, nodes, tt_hits, tt_cutoffs) =
                search.search_best_move(board, depth, history);

            println!(
                "info depth {} score {} nodes {} tthits {} ttcutoffs {}",
                depth, format_uci_score(score), nodes, tt_hits, tt_cutoffs
            );

            match best {
                Some(m) => println!("bestmove {}", m),
                None => println!("bestmove 0000"),
            }
            io::stdout().flush().ok();
            return;
        }
    }

    let max_depth = params.depth.unwrap_or(MAX_DEPTH).min(MAX_DEPTH);

    let ply = history.len().saturating_sub(1) as u32;

    let time_budget = compute_time_budget(
        &params,
        board.side_to_move(),
        ply,
    );

    println!(
        "info string time budget {}ms (ply {}, phase factor {:.2})",
        time_budget.as_millis(),
        ply,
        phase_factor(ply)
    );

    let result =
        search.search_timed(board, max_depth, history, time_budget);

    println!(
        "info depth {} score {} nodes {} tthits {} ttcutoffs {}",
        result.depth_reached,
        format_uci_score(result.score),
        result.nodes,
        result.tt_hits,
        result.tt_cutoffs
    );

    match result.best_move {
        Some(m) => println!("bestmove {}", m),
        None => println!("bestmove 0000"),
    }
    io::stdout().flush().ok();
}

fn handle_setoption(tokens: &[&str], search: &mut Search) {
    if tokens.first() != Some(&"name") {
        return;
    }

    let mut idx = 1;
    let mut name_parts = Vec::new();

    while idx < tokens.len() && tokens[idx] != "value" {
        name_parts.push(tokens[idx]);
        idx += 1;
    }

    let name = name_parts.join(" ");

    let value = if tokens.get(idx) == Some(&"value") {
        tokens.get(idx + 1).copied()
    } else {
        None
    };

    if name == "Threads" {
        if let Some(v) = value.and_then(|v| v.parse::<usize>().ok()) {
            search.set_threads(v.clamp(1, 64));
        }
    }
}

pub fn run() {
    let mut board = Board::default();
    let mut history = GameHistory::new();
    history.push(board.get_hash());
    let mut search = Search::new();
    let stdin = io::stdin();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.is_empty() {
            continue;
        }

        match tokens[0] {
            "uci" => {
                println!("id name {}", ENGINE_NAME);
                println!("id author {}", ENGINE_AUTHOR);
                println!("option name Threads type spin default 4 min 1 max 64");
                println!("uciok");
                io::stdout().flush().ok();
            }
            "isready" => {
                println!("readyok");
                io::stdout().flush().ok();
            }
            "ucinewgame" => {
                board = Board::default();
                history.clear();
                history.push(board.get_hash());
                search.clear_tt();
            }
            "position" => {
                handle_position(
                    &mut board,
                    &mut history,
                    &tokens[1..],
                );
            }
            "go" => {
                handle_go(
                    &board,
                    &mut history,
                    &tokens[1..],
                    &mut search,
                );
            }
            "setoption" => {
                handle_setoption(&tokens[1..], &mut search);
            }
            "quit" => break,
            _ => {}
        }
    }
}