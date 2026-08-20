use std::collections::HashMap;

#[derive(Default)]
pub struct GameHistory {
    stack: Vec<u64>,
    counts: HashMap<u64, u32>,
}

impl GameHistory {
    pub fn new() -> Self {
        Self { stack: Vec::new(), counts: HashMap::new() }
    }

    #[inline]
    pub fn push(&mut self, hash: u64) {
        self.stack.push(hash);
        *self.counts.entry(hash).or_insert(0) += 1;
    }

    #[inline]
    pub fn pop(&mut self) {
        if let Some(hash) = self.stack.pop() {
            if let Some(c) = self.counts.get_mut(&hash) {
                *c -= 1;
                if *c == 0 {
                    self.counts.remove(&hash);
                }
            }
        }
    }

    pub fn clear(&mut self) {
        self.stack.clear();
        self.counts.clear();
    }

    #[inline]
    pub fn count(&self, hash: u64) -> u32 {
        self.counts.get(&hash).copied().unwrap_or(0)
    }

    pub fn len(&self) -> usize {
        self.stack.len()
    }
}