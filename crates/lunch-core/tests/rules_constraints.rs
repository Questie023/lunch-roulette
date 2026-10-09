use std::collections::{BTreeMap, BTreeSet};

use jiff::civil::date;
use lunch_core::model::{DietTag, LunchOffer, Support};
use lunch_core::rules::constraints::{Verdict, check_constraints};

fn set(tags: &[DietTag]) -> BTreeSet<DietTag> {
    tags.iter().copied().collect()
}

fn support(entries: &[(DietTag, Support)]) -> BTreeMap<DietTag, Support> {
    entries.iter().copied().collect()
}

fn offer(tags: &[DietTag]) -> LunchOffer {
    LunchOffer {
        id: 1,
        restaurant_id: 1,
        date: date(2026, 10, 6),
        name: "Dish of the day".into(),
        description: None,
        price_sek: Some(135),
        diet_tags: set(tags),
        source: "test".into(),
    }
}

#[test]
fn no_requirements_is_safe() {
    assert_eq!(check_constraints(&set(&[]), &support(&[]), &[]), Verdict::Safe);
}

#[test]
fn declared_yes_is_safe() {
    let s = support(&[(DietTag::Vegetarian, Support::Yes)]);
    assert_eq!(check_constraints(&set(&[DietTag::Vegetarian]), &s, &[]), Verdict::Safe);
}

#[test]
fn declared_no_is_a_conflict() {
    let s = support(&[(DietTag::Vegan, Support::No)]);
    assert_eq!(
        check_constraints(&set(&[DietTag::Vegan]), &s, &[]),
        Verdict::Conflict(set(&[DietTag::Vegan]))
    );
}

#[test]
fn missing_info_is_unknown_never_safe() {
    assert_eq!(
        check_constraints(&set(&[DietTag::PeanutFree]), &support(&[]), &[]),
        Verdict::Unknown(set(&[DietTag::PeanutFree]))
    );
}

#[test]
fn conflict_wins_over_unknown() {
    let s = support(&[(DietTag::Vegan, Support::No)]);
    let v = check_constraints(&set(&[DietTag::Vegan, DietTag::PeanutFree]), &s, &[]);
    assert_eq!(v, Verdict::Conflict(set(&[DietTag::Vegan])));
}

#[test]
fn offer_with_tag_upgrades_unknown() {
    let o = offer(&[DietTag::Vegetarian]);
    assert_eq!(
        check_constraints(&set(&[DietTag::Vegetarian]), &support(&[]), &[&o]),
        Verdict::Safe
    );
}

#[test]
fn offer_overrides_declared_no() {
    let o = offer(&[DietTag::Vegetarian]);
    let s = support(&[(DietTag::Vegetarian, Support::No)]);
    assert_eq!(
        check_constraints(&set(&[DietTag::Vegetarian]), &s, &[&o]),
        Verdict::Safe
    );
}

#[test]
fn vegan_satisfies_vegetarian() {
    let s = support(&[(DietTag::Vegan, Support::Yes)]);
    assert_eq!(check_constraints(&set(&[DietTag::Vegetarian]), &s, &[]), Verdict::Safe);
}

#[test]
fn vegan_implies_milk_and_egg_free() {
    let s = support(&[(DietTag::Vegan, Support::Yes)]);
    assert_eq!(
        check_constraints(&set(&[DietTag::MilkFree, DietTag::EggFree]), &s, &[]),
        Verdict::Safe
    );
}
