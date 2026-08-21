use chess::{ChessMove, Color};

const HISTORY_MAX: i32 = 1 << 14;

pub(crate) struct HistoryHeuristic {
    table: Box<[[[i32; 64]; 64]; 2]>,
}

impl HistoryHeuristic {
    pub(crate) fn new() -> Self {
        Self {
            table: Box::new([[[0; 64]; 64]; 2]),
        }
    }

    #[inline]
    fn color_index(color: Color) -> usize {
        match color {
            Color::White => 0,
            Color::Black => 1,
        }
    }

    #[inline]
    pub(crate) fn score(&self, color: Color, m: ChessMove) -> i32 {
        let from = m.get_source().to_index();
        let to = m.get_dest().to_index();
        self.table[Self::color_index(color)][from][to]
    }

    pub(crate) fn update(
        &mut self,
        color: Color,
        best: ChessMove,
        tried_quiets: &[ChessMove],
        depth: u32,
    ) {
        let bonus = ((depth * depth) as i32).min(HISTORY_MAX);
        let color_idx = Self::color_index(color);

        for &m in tried_quiets {
            let from = m.get_source().to_index();
            let to = m.get_dest().to_index();
            let delta = if m == best { bonus } else { -bonus };

            let entry = &mut self.table[color_idx][from][to];
            *entry += delta - (*entry * delta.abs()) / HISTORY_MAX;
        }
    }

    pub(crate) fn clear(&mut self) {
        for color_table in self.table.iter_mut() {
            for row in color_table.iter_mut() {
                row.fill(0);
            }
        }
    }
}

#[inline]
pub(crate) fn record_killer(killers: &mut [Option<ChessMove>; 2], m: ChessMove) {
    if killers[0] != Some(m) {
        killers[1] = killers[0];
        killers[0] = Some(m);
    }
}