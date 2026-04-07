//! Sliding window rate limiter.
//!
//! Uses `parking_lot::Mutex` (3–5× faster than `std::sync::Mutex` on
//! uncontended paths). Per-key instantiation means there is zero cross-key
//! lock contention.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use parking_lot::Mutex;

/// A sliding window rate limiter.
///
/// Accurately models OpenAI's actual RPM window semantics. Unlike GCRA
/// (which enforces uniform inter-arrival spacing), a sliding window allows
/// legal bursts within the window. Unlike fixed-bucket schemes, it does not
/// admit boundary bursts.
pub(crate) struct SlidingWindow {
    limit: u32,
    window: Duration,
    timestamps: Mutex<VecDeque<Instant>>,
}

impl SlidingWindow {
    /// Create a new sliding window with the given limit and duration.
    pub(crate) fn new(limit: u32, window: Duration) -> Self {
        Self {
            limit,
            window,
            timestamps: Mutex::new(VecDeque::with_capacity(limit as usize)),
        }
    }

    /// Attempt to record one request. Returns `false` if the window is full.
    pub(crate) fn try_acquire(&self) -> bool {
        let now = Instant::now();
        let cutoff = now - self.window;
        let mut ts = self.timestamps.lock();

        // Evict expired entries from the front.
        while ts.front().is_some_and(|t| *t < cutoff) {
            ts.pop_front();
        }

        if ts.len() < self.limit as usize {
            ts.push_back(now);
            true
        } else {
            false
        }
    }

    /// How many slots remain in the current window (for observability).
    #[allow(dead_code)]
    pub(crate) fn remaining(&self) -> u32 {
        let now = Instant::now();
        let cutoff = now - self.window;
        let ts = self.timestamps.lock();
        let active = ts.iter().filter(|t| **t >= cutoff).count();
        self.limit.saturating_sub(active as u32)
    }
}
