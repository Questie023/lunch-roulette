use jiff::civil::{Time, Weekday, time};
use lunch_core::rules::opening_hours::{Openness, parse_opening_hours};

fn lunch() -> (Time, Time) {
    (time(11, 30, 0, 0), time(13, 30, 0, 0))
}

#[test]
fn weekday_range_is_open_inside_and_closed_outside() {
    let oh = parse_opening_hours("Mo-Fr 11:00-14:00").unwrap();
    assert_eq!(oh.status(Weekday::Wednesday, lunch()), Openness::Open);
    assert_eq!(oh.status(Weekday::Saturday, lunch()), Openness::Closed);
}

#[test]
fn several_rules_separated_by_semicolons() {
    let oh = parse_opening_hours("Tu-Fr 11:00-15:00; Sa 12:00-15:00").unwrap();
    assert_eq!(oh.status(Weekday::Monday, lunch()), Openness::Closed);
    assert_eq!(oh.status(Weekday::Saturday, lunch()), Openness::Open);
}

#[test]
fn garbage_is_an_error() {
    assert!(parse_opening_hours("whenever").is_err());
    assert!(parse_opening_hours("").is_err());
    assert!(parse_opening_hours("Xx 11:00-14:00").is_err());
}

#[test]
fn comma_separated_days() {
    let oh = parse_opening_hours("Mo,We,Fr 11:00-14:00").unwrap();
    assert_eq!(oh.status(Weekday::Wednesday, lunch()), Openness::Open);
    assert_eq!(oh.status(Weekday::Tuesday, lunch()), Openness::Closed);
}

#[test]
fn several_time_ranges() {
    let split = parse_opening_hours("Mo-Fr 07:00-10:00,11:00-14:00").unwrap();
    assert_eq!(split.status(Weekday::Monday, lunch()), Openness::Open);
    let no_lunch = parse_opening_hours("Mo-Fr 07:00-10:00,17:00-22:00").unwrap();
    assert_eq!(no_lunch.status(Weekday::Monday, lunch()), Openness::Closed);
}

#[test]
fn off_overrides_earlier_rule() {
    let oh = parse_opening_hours("Mo-Fr 11:00-14:00; We off").unwrap();
    assert_eq!(oh.status(Weekday::Wednesday, lunch()), Openness::Closed);
    assert_eq!(oh.status(Weekday::Thursday, lunch()), Openness::Open);
}

#[test]
fn needs_45_minutes_inside_window() {
    let late = parse_opening_hours("Mo-Fr 13:00-15:00").unwrap(); // 30 min overlap
    assert_eq!(late.status(Weekday::Monday, lunch()), Openness::Closed);
    let ok = parse_opening_hours("Mo-Fr 12:30-15:00").unwrap(); // 60 min overlap
    assert_eq!(ok.status(Weekday::Monday, lunch()), Openness::Open);
}

#[test]
fn always_open() {
    let oh = parse_opening_hours("24/7").unwrap();
    assert_eq!(oh.status(Weekday::Sunday, lunch()), Openness::Open);
}

#[test]
fn day_range_wraps_week() {
    let oh = parse_opening_hours("Fr-Mo 11:00-14:00").unwrap();
    assert_eq!(oh.status(Weekday::Sunday, lunch()), Openness::Open);
    assert_eq!(oh.status(Weekday::Wednesday, lunch()), Openness::Closed);
}

#[test]
fn tolerates_extra_whitespace() {
    let oh = parse_opening_hours("  Mo-Fr   11:00 - 14:00 ;  ").unwrap();
    assert_eq!(oh.status(Weekday::Monday, lunch()), Openness::Open);
}

#[test]
fn rejects_close_before_open() {
    assert!(parse_opening_hours("Mo 14:00-11:00").is_err());
}
