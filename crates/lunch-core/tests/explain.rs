use lunch_core::model::{FilterStage, Reason, Warning};

#[test]
fn reasons_read_naturally() {
    assert_eq!(
        Reason::CuisineLiked {
            cuisine: "thai".into(),
            count: 1
        }
        .to_string(),
        "1 person likes thai food"
    );
    assert_eq!(
        Reason::CuisineLiked {
            cuisine: "thai".into(),
            count: 3
        }
        .to_string(),
        "3 people like thai food"
    );
    assert_eq!(Reason::Favorite { count: 1 }.to_string(), "a favourite of 1 person");
    assert_eq!(Reason::Favorite { count: 2 }.to_string(), "a favourite of 2 people");
}

#[test]
fn diet_warning_names_nobody() {
    let text = Warning::DietInfoUnknown.to_string();
    assert!(text.contains("dietary"));
}

#[test]
fn every_empty_stage_has_a_message() {
    for stage in [
        FilterStage::NoRestaurants,
        FilterStage::AllShown,
        FilterStage::Closed,
        FilterStage::TooFar,
        FilterStage::DietConflict,
    ] {
        assert!(!stage.to_string().is_empty());
    }
}

#[test]
fn liked_before_reads_naturally() {
    assert_eq!(
        Reason::LikedBefore { liked: 2, of: 3 }.to_string(),
        "2 of 3 liked it last time"
    );
}
