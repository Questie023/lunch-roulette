use lunch_core::rules::roulette::pick;
use rand::SeedableRng;
use rand::rngs::StdRng;

#[test]
fn empty_is_none() {
    let mut rng = StdRng::seed_from_u64(1);
    assert_eq!(pick(&[], &mut rng), None);
}

#[test]
fn single_candidate_is_chosen() {
    let mut rng = StdRng::seed_from_u64(1);
    assert_eq!(pick(&[2.5], &mut rng), Some(0));
}

#[test]
fn index_is_always_in_range() {
    let mut rng = StdRng::seed_from_u64(1);
    for _ in 0..1_000 {
        assert!(pick(&[1.0, 2.0, 3.0], &mut rng).unwrap() < 3);
    }
}

#[test]
fn zero_weight_never_chosen() {
    let mut rng = StdRng::seed_from_u64(2);
    for _ in 0..10_000 {
        assert_eq!(pick(&[0.0, 1.0], &mut rng), Some(1));
    }
}

#[test]
fn only_non_positive_or_nan_is_none() {
    let mut rng = StdRng::seed_from_u64(3);
    assert_eq!(pick(&[0.0, -1.0, f64::NAN], &mut rng), None);
}

#[test]
fn proportional_to_weight() {
    let mut rng = StdRng::seed_from_u64(7);
    let draws = 40_000;
    let ones = (0..draws).filter(|_| pick(&[1.0, 3.0], &mut rng) == Some(1)).count();
    let share = ones as f64 / draws as f64;
    assert!((share - 0.75).abs() < 0.02, "share {share}");
}
