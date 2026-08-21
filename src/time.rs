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

#[inline]
fn opponent_time_multiplier(
    own_time_ms: u64,
    opponent_time_ms: u64,
) -> f64 {
    if own_time_ms == 0 {
        return 0.55;
    }

    let own = own_time_ms as f64;
    let opponent = opponent_time_ms as f64;

    let ratio = (opponent / own).clamp(0.25, 4.0);

    let mut factor = if ratio > 1.0 {
        1.0 - ratio.ln() * 0.18
    } else {
        1.0 + (1.0 / ratio).ln() * 0.12
    };

    if own_time_ms < 10_000 && opponent_time_ms > 30_000 {
        factor *= 0.75;
    }

    if own_time_ms < 5_000 {
        factor *= 0.65;
    }

    factor.clamp(0.55, 1.15)
}

#[inline]
fn moves_to_go(
    params: &GoParams,
    time_left: u64,
) -> u64 {
    if let Some(moves) = params.movestogo {
        return moves.max(1);
    }

    if time_left < 10_000 {
        12
    } else if time_left < 30_000 {
        20
    } else {
        30
    }
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

    let (time_left, opponent_time, increment) =
        match side_to_move {
            Color::White => (
                params.wtime_ms,
                params.btime_ms,
                params.winc_ms,
            ),

            Color::Black => (
                params.btime_ms,
                params.wtime_ms,
                params.binc_ms,
            ),
        };

    let Some(time_left) = time_left else {
        return Duration::from_millis(DEFAULT_MOVETIME_MS);
    };

    let opponent_time =
        opponent_time.unwrap_or(time_left);

    let moves_to_go =
        moves_to_go(params, time_left);

    let base = time_left / moves_to_go;

    let allocated =
        base + increment / 2;

    let phase =
        phase_factor(ply);

    let opponent_factor =
        opponent_time_multiplier(
            time_left,
            opponent_time,
        );

    let scaled =
        (allocated as f64
            * phase
            * opponent_factor) as u64;

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