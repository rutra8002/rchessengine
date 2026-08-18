use crate::engine::{search_best_move, DEFAULT_DEPTH};
use chess::{Board, ChessMove, Piece, Square};
use std::io::{self, BufRead, Write};
use std::str::FromStr;

const ENGINE_NAME: &str = "rchessengine";
const ENGINE_AUTHOR: &str = "ruter";

fn handle_position(board: &mut Board, tokens: &[&str]) {
    let mut idx = 0;

    if tokens.get(idx) == Some(&"startpos") {
        *board = Board::default();
        idx += 1;
    } else if tokens.get(idx) == Some(&"fen") {
        idx += 1;
        let fen_fields: Vec<&str> = tokens[idx..].iter().take(6).cloned().collect();
        let fen = fen_fields.join(" ");
        idx += fen_fields.len();
        if let Ok(b) = Board::from_str(&fen) {
            *board = b;
        }
    }

    if tokens.get(idx) == Some(&"moves") {
        idx += 1;
        for mv_str in &tokens[idx..] {
            match parse_uci_move(board, mv_str) {
                Some(m) if board.legal(m) => {
                    *board = board.make_move_new(m);
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

fn handle_go(board: &Board, tokens: &[&str]) {
    let mut depth = DEFAULT_DEPTH;
    let mut i = 0;
    while i < tokens.len() {
        if tokens[i] == "depth" {
            if let Some(d) = tokens.get(i + 1).and_then(|t| t.parse::<u32>().ok()) {
                depth = d.max(1);
            }
        }
        i += 1;
    }

    let (best, score, nodes) = search_best_move(board, depth);

    eprintln!("info depth {} score cp {} nodes {}", depth, score, nodes);

    match best {
        Some(m) => println!("bestmove {}", m),
        None => println!("bestmove 0000"),
    }
    io::stdout().flush().ok();
}

pub fn run() {
    let mut board = Board::default();
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
            }
            "position" => {
                handle_position(&mut board, &tokens[1..]);
            }
            "go" => {
                handle_go(&board, &tokens[1..]);
            }
            "quit" => break,
            _ => {}
        }
    }
}