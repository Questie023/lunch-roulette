//! Variety: a multiplier in (0, 1] by days since the restaurant was last chosen, linear between
//! the anchors below.

const ANCHORS: [(u32, f64); 5] = [(0, 0.05), (1, 0.1), (2, 0.4), (7, 0.85), (14, 1.0)];

pub fn variety_multiplier(days_since_last: Option<u32>) -> f64 {
    let Some(days) = days_since_last else {
        return 1.0;
    };
    for pair in ANCHORS.windows(2) {
        let ((d0, m0), (d1, m1)) = (pair[0], pair[1]);
        if days <= d1 {
            let t = f64::from(days - d0) / f64::from(d1 - d0);
            return m0 + t * (m1 - m0);
        }
    }
    1.0
}
