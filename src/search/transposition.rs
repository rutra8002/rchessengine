use chess::{ChessMove, File, Piece, Rank, Square};
use std::sync::atomic::{AtomicU64, Ordering};

const MATE_SCORE: i32 = 67_000_000;
const MATE_IN_MAX_PLY: i32 = MATE_SCORE - 1000;

const OCCUPIED_BIT: u64 = 1 << 63;

pub(crate) const DEFAULT_HASH_MB: usize = 256;

const MIN_HASH_SLOTS: usize = 1024;

#[inline]
pub(crate) fn score_to_tt(score: i32, ply: i32) -> i32 {
    if score >= MATE_IN_MAX_PLY {
        score + ply
    } else if score <= -MATE_IN_MAX_PLY {
        score - ply
    } else {
        score
    }
}

#[inline]
pub(crate) fn score_from_tt(score: i32, ply: i32) -> i32 {
    if score >= MATE_IN_MAX_PLY {
        score - ply
    } else if score <= -MATE_IN_MAX_PLY {
        score + ply
    } else {
        score
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub(crate) enum Bound {
    Exact = 0,
    Lower = 1,
    Upper = 2,
}

impl Bound {
    #[inline]
    fn from_u8(v: u8) -> Bound {
        match v {
            1 => Bound::Lower,
            2 => Bound::Upper,
            _ => Bound::Exact,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct TTEntry {
    pub(crate) depth: i16,
    pub(crate) score: i32,
    pub(crate) bound: Bound,
    best_move_bits: u16,
}

impl TTEntry {
    #[inline]
    pub(crate) fn best_move(self) -> Option<ChessMove> {
        decode_move(self.best_move_bits)
    }
}

#[inline]
fn encode_move(m: Option<ChessMove>) -> u16 {
    match m {
        None => 0xFFFF,
        Some(mv) => {
            let from = mv.get_source().to_index() as u16 & 0x3F;
            let to = mv.get_dest().to_index() as u16 & 0x3F;

            let promo: u16 = match mv.get_promotion() {
                None => 0,
                Some(Piece::Knight) => 1,
                Some(Piece::Bishop) => 2,
                Some(Piece::Rook) => 3,
                Some(Piece::Queen) => 4,
                _ => 0,
            };

            from | (to << 6) | (promo << 12)
        }
    }
}

#[inline]
fn decode_move(bits: u16) -> Option<ChessMove> {
    if bits == 0xFFFF {
        return None;
    }

    let from = (bits & 0x3F) as usize;
    let to = ((bits >> 6) & 0x3F) as usize;
    let promo = (bits >> 12) & 0x7;

    let promotion = match promo {
        1 => Some(Piece::Knight),
        2 => Some(Piece::Bishop),
        3 => Some(Piece::Rook),
        4 => Some(Piece::Queen),
        _ => None,
    };

    let source =
        Square::make_square(Rank::from_index(from / 8), File::from_index(from % 8));
    let dest =
        Square::make_square(Rank::from_index(to / 8), File::from_index(to % 8));

    Some(ChessMove::new(source, dest, promotion))
}

#[inline]
fn pack(depth: u32, score: i32, bound: Bound, best_move: Option<ChessMove>) -> u64 {
    let mv = encode_move(best_move) as u64;
    let score_bits = (score as u32) as u64;
    let depth_bits = (depth.min(255) as u64) & 0xFF;
    let bound_bits = (bound as u64) & 0x3;

    OCCUPIED_BIT
        | (bound_bits << 56)
        | (depth_bits << 48)
        | (score_bits << 16)
        | mv
}

#[inline]
fn unpack(data: u64) -> TTEntry {
    let mv_bits = (data & 0xFFFF) as u16;
    let score_bits = ((data >> 16) & 0xFFFF_FFFF) as u32;
    let depth_bits = ((data >> 48) & 0xFF) as u8;
    let bound_bits = ((data >> 56) & 0x3) as u8;

    TTEntry {
        depth: depth_bits as i16,
        score: score_bits as i32,
        bound: Bound::from_u8(bound_bits),
        best_move_bits: mv_bits,
    }
}

struct Slot {
    key_xor_data: AtomicU64,
    data: AtomicU64,
}

impl Default for Slot {
    fn default() -> Self {
        Self {
            key_xor_data: AtomicU64::new(0),
            data: AtomicU64::new(0),
        }
    }
}

pub(crate) struct TranspositionTable {
    entries: Vec<Slot>,
    mask: usize,
}

impl TranspositionTable {
    pub(crate) fn with_size_mb(mb: usize) -> Self {
        let slots = Self::slots_for_mb(mb);

        let mut entries = Vec::with_capacity(slots);
        entries.resize_with(slots, Slot::default);

        Self {
            entries,
            mask: slots - 1,
        }
    }

    pub(crate) fn new() -> Self {
        Self::with_size_mb(DEFAULT_HASH_MB)
    }

    fn slots_for_mb(mb: usize) -> usize {
        let bytes = (mb.max(1) as u64) * 1024 * 1024;
        let slot_size = std::mem::size_of::<Slot>() as u64;
        let raw_slots = (bytes / slot_size).max(1);

        let mut slots: u64 = 1;
        while slots * 2 <= raw_slots {
            slots *= 2;
        }

        (slots as usize).max(MIN_HASH_SLOTS)
    }

    #[inline]
    fn index(&self, key: u64) -> usize {
        (key as usize) & self.mask
    }

    #[inline]
    pub(crate) fn probe(&self, key: u64) -> Option<TTEntry> {
        let slot = &self.entries[self.index(key)];

        let data = slot.data.load(Ordering::Relaxed);

        if data & OCCUPIED_BIT == 0 {
            return None;
        }

        let key_xor_data = slot.key_xor_data.load(Ordering::Relaxed);

        if (key_xor_data ^ data) != key {
            return None;
        }

        Some(unpack(data))
    }

    #[inline]
    pub(crate) fn store(
        &self,
        key: u64,
        depth: u32,
        score: i32,
        bound: Bound,
        best_move: Option<ChessMove>,
    ) {
        let slot = &self.entries[self.index(key)];

        let existing_data = slot.data.load(Ordering::Relaxed);

        if existing_data & OCCUPIED_BIT != 0 {
            let existing_kxd = slot.key_xor_data.load(Ordering::Relaxed);

            if (existing_kxd ^ existing_data) == key {
                let existing_depth = (existing_data >> 48) & 0xFF;
                let existing_bound =
                    Bound::from_u8(((existing_data >> 56) & 0x3) as u8);

                if existing_depth as u32 > depth
                    || (existing_depth as u32 == depth
                    && existing_bound == Bound::Exact
                    && bound != Bound::Exact)
                {
                    return;
                }
            }
        }

        let data = pack(depth, score, bound, best_move);

        slot.data.store(data, Ordering::Relaxed);
        slot.key_xor_data.store(key ^ data, Ordering::Relaxed);
    }

    pub(crate) fn clear(&self) {
        for slot in &self.entries {
            slot.data.store(0, Ordering::Relaxed);
            slot.key_xor_data.store(0, Ordering::Relaxed);
        }
    }
}