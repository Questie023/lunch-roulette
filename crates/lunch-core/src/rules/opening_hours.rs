//! Opening hours in a subset of the OpenStreetMap `opening_hours` syntax:
//! `Mo-Fr 11:00-14:00; Sa 12:00-15:00`, day lists (`Mo,We,Fr`), ranges that wrap the week
//! (`Fr-Mo`), several time ranges (`07:00-10:00,11:00-14:00`), `off`, and `24/7`.
//! Later rules replace earlier ones for the days they name, as in OSM.

use jiff::civil::{Time, Weekday};

/// A place counts as open when it is open at least this long inside the lunch window.
const MIN_OVERLAP_MIN: i32 = 45;
const DAY_MIN: u16 = 24 * 60;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Openness {
    Open,
    Closed,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("cannot parse opening hours {input:?}: {reason}")]
pub struct OhError {
    pub input: String,
    pub reason: String,
}

/// Minutes since midnight, `open < close ≤ 24:00`, per weekday (Monday = 0).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpeningHours {
    days: [Vec<(u16, u16)>; 7],
}

const DAYS: [&str; 7] = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];

pub fn parse_opening_hours(input: &str) -> Result<OpeningHours, OhError> {
    let err = |reason: &str| OhError {
        input: input.to_string(),
        reason: reason.to_string(),
    };
    let mut days: [Vec<(u16, u16)>; 7] = Default::default();
    let mut rules = 0;
    for rule in input.split(';') {
        let rule: String = rule.chars().filter(|c| !c.is_whitespace()).collect();
        if rule.is_empty() {
            continue;
        }
        rules += 1;
        if rule == "24/7" {
            days = std::array::from_fn(|_| vec![(0, DAY_MIN)]);
            continue;
        }
        if let Some(day_part) = rule.strip_suffix("off") {
            for d in parse_days(day_part).map_err(&err)? {
                days[d].clear();
            }
            continue;
        }
        let split = rule
            .find(|c: char| c.is_ascii_digit())
            .ok_or_else(|| err("expected opening times"))?;
        let (day_part, time_part) = rule.split_at(split);
        let ranges = time_part
            .split(',')
            .map(parse_range)
            .collect::<Result<Vec<_>, _>>()
            .map_err(&err)?;
        for d in parse_days(day_part).map_err(&err)? {
            days[d] = ranges.clone();
        }
    }
    if rules == 0 {
        return Err(err("no rules"));
    }
    Ok(OpeningHours { days })
}

fn day_index(name: &str) -> Result<usize, &'static str> {
    DAYS.iter().position(|d| *d == name).ok_or("unknown day")
}

/// `Mo`, `Mo-Fr`, `Fr-Mo` (wraps), and comma-separated lists of those.
fn parse_days(spec: &str) -> Result<Vec<usize>, &'static str> {
    if spec.is_empty() {
        return Err("expected days");
    }
    let mut out = Vec::new();
    for item in spec.split(',') {
        match item.split_once('-') {
            Some((a, b)) => {
                let (from, to) = (day_index(a)?, day_index(b)?);
                let len = (to + 7 - from) % 7 + 1;
                out.extend((0..len).map(|k| (from + k) % 7));
            }
            None => out.push(day_index(item)?),
        }
    }
    Ok(out)
}

fn parse_range(range: &str) -> Result<(u16, u16), &'static str> {
    let (open, close) = range.split_once('-').ok_or("expected HH:MM-HH:MM")?;
    let (open, close) = (parse_minutes(open)?, parse_minutes(close)?);
    if close <= open {
        return Err("closing time must be after opening time");
    }
    Ok((open, close))
}

fn parse_minutes(hhmm: &str) -> Result<u16, &'static str> {
    let (h, m) = hhmm.split_once(':').ok_or("expected HH:MM")?;
    let (h, m): (u16, u16) = (h.parse().map_err(|_| "bad hour")?, m.parse().map_err(|_| "bad minute")?);
    if m >= 60 || h > 24 || (h == 24 && m != 0) {
        return Err("time out of range");
    }
    Ok(h * 60 + m)
}

fn minutes(t: Time) -> i32 {
    i32::from(t.hour()) * 60 + i32::from(t.minute())
}

impl OpeningHours {
    pub fn status(&self, weekday: Weekday, window: (Time, Time)) -> Openness {
        let (from, to) = (minutes(window.0), minutes(window.1));
        let needed = MIN_OVERLAP_MIN.min(to - from).max(1);
        let day = usize::try_from(weekday.to_monday_zero_offset()).unwrap_or(0);
        let overlap: i32 = self.days[day]
            .iter()
            .map(|&(open, close)| (i32::from(close).min(to) - i32::from(open).max(from)).max(0))
            .sum();
        if overlap >= needed {
            Openness::Open
        } else {
            Openness::Closed
        }
    }
}
