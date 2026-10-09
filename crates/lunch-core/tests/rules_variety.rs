use lunch_core::rules::variety::variety_multiplier;

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn anchors_from_the_spec() {
    assert!(close(variety_multiplier(None), 1.0));
    assert!(close(variety_multiplier(Some(1)), 0.1));
    assert!(close(variety_multiplier(Some(2)), 0.4));
    assert!(close(variety_multiplier(Some(7)), 0.85));
    assert!(close(variety_multiplier(Some(14)), 1.0));
    assert!(close(variety_multiplier(Some(365)), 1.0));
}

#[test]
fn always_positive_and_monotonic() {
    let mut previous = 0.0;
    for d in 0..=30 {
        let m = variety_multiplier(Some(d));
        assert!(m > 0.0 && m <= 1.0, "day {d}: {m}");
        assert!(m >= previous, "day {d} decreased");
        previous = m;
    }
}

#[test]
fn same_day_is_strongest() {
    assert!(close(variety_multiplier(Some(0)), 0.05));
}

#[test]
fn linear_between_anchors() {
    assert!(close(variety_multiplier(Some(4)), 0.58));
    assert!(close(variety_multiplier(Some(10)), 0.85 + 3.0 / 7.0 * 0.15));
}
