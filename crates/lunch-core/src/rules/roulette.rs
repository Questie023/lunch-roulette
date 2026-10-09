//! Weighted random choice: index `i` wins with probability `weights[i] / Σ weights`.
//! Weights that are not finite or not positive never win.

use rand::{Rng, RngExt};

pub fn pick<R: Rng + ?Sized>(weights: &[f64], rng: &mut R) -> Option<usize> {
    let usable = |w: f64| w.is_finite() && w > 0.0;
    let total: f64 = weights.iter().copied().filter(|w| usable(*w)).sum();
    if !(total.is_finite() && total > 0.0) {
        return None;
    }
    let mut target = rng.random_range(0.0..total);
    let mut last = None;
    for (i, &w) in weights.iter().enumerate() {
        if !usable(w) {
            continue;
        }
        if target < w {
            return Some(i);
        }
        target -= w;
        last = Some(i);
    }
    // Floating-point rounding can leave a sliver past the last usable weight.
    last
}
