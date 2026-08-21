use chess::Color;
use std::time::{Duration, Instant};

const SAFETY_MARGIN_MS: u64 = 50;
const DEFAULT_MOVETIME_MS: u64 = 5000;
const MAX_TIME_FRACTION: f64 = 0.33;

#[derive(Debug, Clone, Copy, Default)]
pub struct GoParams {
    pub depth: Option<u32>,
    pub movetime_ms: Option<u64>,

    pub wtime_ms: Option<u64>,
    pub btime_ms: Option<u64>,

    pub winc_ms: u64,
    pub binc_ms: u64,

    pub movestogo: Option<u64>,
}

pub fn parse_go_params(tokens: &[&str]) -> GoParams {
    let mut params = GoParams::default();

    let mut i = 0;

    while i < tokens.len() {
        match tokens[i] {
            "depth" => {
                params.depth =
                    tokens.get(i + 1).and_then(|v| v.parse().ok());
                i += 1;
            }

            "movetime" => {
                params.movetime_ms =
                    tokens.get(i + 1).and_then(|v| v.parse().ok());
                i += 1;
            }

            "wtime" => {
                params.wtime_ms =
                    tokens.get(i + 1).and_then(|v| v.parse().ok());
                i += 1;
            }

            "btime" => {
                params.btime_ms =
                    tokens.get(i + 1).and_then(|v| v.parse().ok());
                i += 1;
            }

            "winc" => {
                params.winc_ms = tokens
                    .get(i + 1)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0);

                i += 1;
            }

            "binc" => {
                params.binc_ms = tokens
                    .get(i + 1)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0);

                i += 1;
            }

            "movestogo" => {
                params.movestogo =
                    tokens.get(i + 1).and_then(|v| v.parse().ok());

                i += 1;
            }

            _ => {}
        }

        i += 1;
    }

    params
}

#[inline]
pub fn phase_factor(ply: u32) -> f64 {
    let ply = ply as f64;

    0.85 + 0.75 / (1.0 + ply / 18.0)
}

pub fn compute_time_budget(
    params: &GoParams,
    side_to_move: Color,
    ply: u32,
) -> Duration {
    if let Some(movetime) = params.movetime_ms {
        let budget = movetime
            .saturating_sub(SAFETY_MARGIN_MS)
            .max(1);

        return Duration::from_millis(budget);
    }

    let (time_left, increment) = match side_to_move {
        Color::White => (
            params.wtime_ms,
            params.winc_ms,
        ),

        Color::Black => (
            params.btime_ms,
            params.binc_ms,
        ),
    };

    let Some(time_left) = time_left else {
        return Duration::from_millis(DEFAULT_MOVETIME_MS);
    };

    let moves_to_go =
        params.movestogo.unwrap_or(30).max(1);

    let base = time_left / moves_to_go;

    let allocated =
        base + increment / 2;

    let scaled =
        (allocated as f64 * phase_factor(ply)) as u64;

    let safety_cap =
        time_left.saturating_sub(SAFETY_MARGIN_MS);

    let hard_cap =
        (time_left as f64 * MAX_TIME_FRACTION) as u64;

    let budget =
        scaled
            .min(safety_cap)
            .min(hard_cap)
            .max(1);

    Duration::from_millis(budget)
}

#[derive(Debug, Clone, Copy)]
pub struct SearchDeadline {
    deadline: Instant,
}

impl SearchDeadline {
    #[inline]
    pub fn new(budget: Duration) -> Self {
        Self {
            deadline: Instant::now() + budget,
        }
    }

    #[inline]
    pub fn expired(&self) -> bool {
        Instant::now() >= self.deadline
    }

    #[inline]
    pub fn remaining(&self) -> Duration {
        self.deadline
            .saturating_duration_since(Instant::now())
    }

    #[inline]
    pub fn instant(&self) -> Instant {
        self.deadline
    }
}