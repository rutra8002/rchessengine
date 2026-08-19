use crate::search::{Search, MAX_DEPTH};
use chess::{Board, ChessMove, Color, Piece, Square};
use std::io::{self, BufRead, Write};
use std::str::FromStr;
use std::time::Duration;

const ENGINE_NAME: &str = "rchessengine";
const ENGINE_AUTHOR: &str = "ruter";

const SAFETY_MARGIN_MS: u64 = 50;

const DEFAULT_MOVETIME_MS: u64 = 5000;

fn handle_position(board: &mut Board, history: &mut Vec<u64>, tokens: &[&str]) {
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

struct GoParams {
    depth: Option<u32>,
    movetime_ms: Option<u64>,
    wtime_ms: Option<u64>,
    btime_ms: Option<u64>,
    winc_ms: u64,
    binc_ms: u64,
    movestogo: Option<u64>,
}

fn parse_go_params(tokens: &[&str]) -> GoParams {
    let mut params = GoParams {
        depth: None,
        movetime_ms: None,
        wtime_ms: None,
        btime_ms: None,
        winc_ms: 0,
        binc_ms: 0,
        movestogo: None,
    };

    let mut i = 0;
    while i < tokens.len() {
        match tokens[i] {
            "depth" => {
                params.depth =
                    tokens.get(i + 1).and_then(|t| t.parse().ok());
                i += 1;
            }
            "movetime" => {
                params.movetime_ms =
                    tokens.get(i + 1).and_then(|t| t.parse().ok());
                i += 1;
            }
            "wtime" => {
                params.wtime_ms =
                    tokens.get(i + 1).and_then(|t| t.parse().ok());
                i += 1;
            }
            "btime" => {
                params.btime_ms =
                    tokens.get(i + 1).and_then(|t| t.parse().ok());
                i += 1;
            }
            "winc" => {
                params.winc_ms = tokens
                    .get(i + 1)
                    .and_then(|t| t.parse().ok())
                    .unwrap_or(0);
                i += 1;
            }
            "binc" => {
                params.binc_ms = tokens
                    .get(i + 1)
                    .and_then(|t| t.parse().ok())
                    .unwrap_or(0);
                i += 1;
            }
            "movestogo" => {
                params.movestogo =
                    tokens.get(i + 1).and_then(|t| t.parse().ok());
                i += 1;
            }
            _ => {}
        }
        i += 1;
    }

    params
}

fn compute_time_budget(
    params: &GoParams,
    side_to_move: Color,
) -> Option<Duration> {
    if let Some(mt) = params.movetime_ms {
        let budget = mt.saturating_sub(SAFETY_MARGIN_MS).max(1);
        return Some(Duration::from_millis(budget));
    }

    let (time_left, inc) = match side_to_move {
        Color::White => (params.wtime_ms, params.winc_ms),
        Color::Black => (params.btime_ms, params.binc_ms),
    };

    let time_left = time_left?;

    let moves_to_go = params.movestogo.unwrap_or(30).max(1);

    let base = time_left / moves_to_go;
    let allocated = base + inc / 2;

    let safety_cap = time_left.saturating_sub(SAFETY_MARGIN_MS);
    let budget_ms = allocated.min(safety_cap).max(1);

    Some(Duration::from_millis(budget_ms))
}

fn handle_go(
    board: &Board,
    history: &mut Vec<u64>,
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

            eprintln!(
                "info depth {} score cp {} nodes {} tthits {} ttcutoffs {}",
                depth, score, nodes, tt_hits, tt_cutoffs
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

    let time_budget = compute_time_budget(&params, board.side_to_move())
        .unwrap_or_else(|| Duration::from_millis(DEFAULT_MOVETIME_MS));

    let result =
        search.search_timed(board, max_depth, history, time_budget);

    eprintln!(
        "info depth {} score cp {} nodes {} tthits {} ttcutoffs {}",
        result.depth_reached,
        result.score,
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

pub fn run() {
    let mut board = Board::default();
    let mut history: Vec<u64> = vec![board.get_hash()];
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
            "quit" => break,
            _ => {}
        }
    }
}