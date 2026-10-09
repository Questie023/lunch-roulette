//! Hard dietary constraints. A known conflict removes a restaurant; unknown never counts as safe.

use std::collections::{BTreeMap, BTreeSet};

use crate::model::{DietTag, LunchOffer, Support};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    Safe,
    Unknown(BTreeSet<DietTag>),
    Conflict(BTreeSet<DietTag>),
}

/// Tags that also satisfy `tag`: a vegan option is vegetarian, milk-free and egg-free.
fn satisfied_by(tag: DietTag) -> Vec<DietTag> {
    match tag {
        DietTag::Vegetarian | DietTag::MilkFree | DietTag::EggFree => vec![tag, DietTag::Vegan],
        _ => vec![tag],
    }
}

/// A requirement is met by a declared `Yes` (directly or via an implying tag) or by one of today's
/// offers. A declared `No` with no offer to rescue it is a conflict; anything else is unknown.
pub fn check_constraints(
    required: &BTreeSet<DietTag>,
    support: &BTreeMap<DietTag, Support>,
    offers_today: &[&LunchOffer],
) -> Verdict {
    let mut unknown = BTreeSet::new();
    let mut conflict = BTreeSet::new();
    for &tag in required {
        let options = satisfied_by(tag);
        let declared = options.iter().any(|t| support.get(t) == Some(&Support::Yes));
        let offered = offers_today
            .iter()
            .any(|o| options.iter().any(|t| o.diet_tags.contains(t)));
        if declared || offered {
            continue;
        }
        if support.get(&tag) == Some(&Support::No) {
            conflict.insert(tag);
        } else {
            unknown.insert(tag);
        }
    }
    if !conflict.is_empty() {
        Verdict::Conflict(conflict)
    } else if !unknown.is_empty() {
        Verdict::Unknown(unknown)
    } else {
        Verdict::Safe
    }
}
