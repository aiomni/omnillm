//! Fixed-point, lock-free budget tracker with two-phase settle.

use std::sync::atomic::{AtomicU64, Ordering};

/// Fixed-point cost representation.
///
/// 1 USD = 1,000,000 micro-dollars.
/// Using `u64` avoids float precision loss and enables lock-free CAS.
pub type MicroDollar = u64;

/// A lock-free budget tracker using two-phase pre-reservation and settlement.
///
/// Costs are stored as [`MicroDollar`] (1 USD = 1,000,000 units) to avoid
/// floating-point precision loss and enable atomic CAS operations.
///
/// # Example
///
/// ```
/// use omni_gateway::BudgetTracker;
///
/// let tracker = BudgetTracker::new(50.0);
/// assert!(tracker.try_reserve(1_000_000)); // reserve $1.00
/// tracker.settle(1_000_000, 500_000);      // actual was $0.50, refund $0.50
/// assert!((tracker.used_usd() - 0.5).abs() < 0.001);
/// ```
pub struct BudgetTracker {
    limit: MicroDollar,
    used: AtomicU64,
}

impl BudgetTracker {
    /// Create a new budget tracker with the given limit in USD.
    pub fn new(limit_usd: f64) -> Self {
        Self {
            limit: usd_to_micro(limit_usd),
            used: AtomicU64::new(0),
        }
    }

    /// Pre-occupy `estimated` micro-dollars.
    ///
    /// Returns `false` without modifying state if the budget would be exceeded.
    pub fn try_reserve(&self, estimated: MicroDollar) -> bool {
        loop {
            let cur = self.used.load(Ordering::Acquire);
            if cur + estimated > self.limit {
                return false;
            }
            match self.used.compare_exchange_weak(
                cur,
                cur + estimated,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(_) => return true,
                Err(_) => std::hint::spin_loop(),
            }
        }
    }

    /// Settle the difference between pre-estimated and actual cost.
    ///
    /// Called after the provider response arrives.
    /// Pass `actual = 0` on error for a full refund of the reservation.
    pub fn settle(&self, estimated: MicroDollar, actual: MicroDollar) {
        if actual > estimated {
            self.used.fetch_add(actual - estimated, Ordering::Relaxed);
        } else {
            self.used.fetch_sub(estimated - actual, Ordering::Relaxed);
        }
    }

    /// Returns the total cost consumed so far, in USD.
    pub fn used_usd(&self) -> f64 {
        micro_to_usd(self.used.load(Ordering::Relaxed))
    }

    /// Returns the configured budget limit, in USD.
    pub fn limit_usd(&self) -> f64 {
        micro_to_usd(self.limit)
    }

    /// Returns the remaining budget, in USD.
    pub fn remaining_usd(&self) -> f64 {
        let used = self.used.load(Ordering::Relaxed);
        micro_to_usd(self.limit.saturating_sub(used))
    }
}

/// Convert a USD amount to micro-dollars.
#[inline]
pub fn usd_to_micro(usd: f64) -> MicroDollar {
    (usd * 1_000_000.0) as u64
}

/// Convert micro-dollars to a USD amount.
#[inline]
pub fn micro_to_usd(micro: MicroDollar) -> f64 {
    micro as f64 / 1_000_000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserve_and_settle() {
        let tracker = BudgetTracker::new(10.0);
        let est = usd_to_micro(3.0);
        assert!(tracker.try_reserve(est));
        assert!((tracker.used_usd() - 3.0).abs() < 0.001);

        // Actual was cheaper — refund the delta.
        let actual = usd_to_micro(1.5);
        tracker.settle(est, actual);
        assert!((tracker.used_usd() - 1.5).abs() < 0.001);
    }

    #[test]
    fn budget_exceeded() {
        let tracker = BudgetTracker::new(1.0);
        let est = usd_to_micro(2.0);
        assert!(!tracker.try_reserve(est));
        assert!((tracker.used_usd() - 0.0).abs() < 0.001);
    }

    #[test]
    fn full_refund_on_error() {
        let tracker = BudgetTracker::new(10.0);
        let est = usd_to_micro(5.0);
        assert!(tracker.try_reserve(est));
        tracker.settle(est, 0); // error — full refund
        assert!((tracker.used_usd() - 0.0).abs() < 0.001);
    }
}
