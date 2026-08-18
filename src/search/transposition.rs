use chess::ChessMove;

const TT_BITS: usize = 20;
const TT_SIZE: usize = 1 << TT_BITS;
const TT_MASK: usize = TT_SIZE - 1;

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum Bound {
    Exact = 0,
    Lower = 1,
    Upper = 2,
}

#[derive(Clone, Copy)]
pub(crate) struct TTEntry {
    pub(crate) key: u64,
    pub(crate) depth: i16,
    pub(crate) score: i32,
    pub(crate) bound: Bound,
    pub(crate) best_move: Option<ChessMove>,
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

pub(crate) struct TranspositionTable {
    entries: Vec<TTEntry>,
}

impl TranspositionTable {
    pub(crate) fn new() -> Self {
        Self {
            entries: vec![
                TTEntry::default();
                TT_SIZE
            ],
        }
    }

    #[inline]
    fn index(key: u64) -> usize {
        (key as usize) & TT_MASK
    }

    #[inline]
    pub(crate) fn probe(
        &self,
        key: u64,
    ) -> Option<&TTEntry> {
        let entry = &self.entries[Self::index(key)];

        if entry.depth >= 0 && entry.key == key {
            Some(entry)
        } else {
            None
        }
    }

    #[inline]
    pub(crate) fn store(
        &mut self,
        key: u64,
        depth: u32,
        score: i32,
        bound: Bound,
        best_move: Option<ChessMove>,
    ) {
        let index = Self::index(key);
        let old = self.entries[index];

        if old.depth < depth as i16
            || old.key != key
        {
            self.entries[index] = TTEntry {
                key,
                depth: depth as i16,
                score,
                bound,
                best_move,
            };
        }
    }

    pub(crate) fn clear(&mut self) {
        self.entries.fill(TTEntry::default());
    }
}