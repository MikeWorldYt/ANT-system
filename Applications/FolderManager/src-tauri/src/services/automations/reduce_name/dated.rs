// A date in the name moves to the front as "(yy-mm-dd)_" or "(yy-mm-dd_hh-mm)_", so names sort by when they were made. Port of the Reformatter's dated_name.
use std::sync::LazyLock;

use regex::{Captures, Regex, RegexBuilder};

const CAMERA: &str = r"(?:(?:PXL|IMG|VID)[-_])?";
static DATE: LazyLock<Regex> = LazyLock::new(|| Regex::new(&format!(r"(?:^|[^0-9])({CAMERA}(20\d{{2}})[-_]?(0[1-9]|1[0-2])[-_]?([0-2]\d|3[01]))(?:[^0-9]|$)")).expect("valid date regex"));
// "20240905121359" glues the time to the date with no separator.
static COMPACT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(r"(?:^|[^0-9])({CAMERA}(20\d{{2}})(0[1-9]|1[0-2])([0-2]\d|3[01])([01]\d|2[0-3])([0-5]\d)[0-5]\d)(?:[^0-9]|$)")).expect("valid compact regex")
});
// "27 sept 2026", as ChatGPT and other apps write it in the user's language.
static NAMED: LazyLock<Regex> = LazyLock::new(|| {
    RegexBuilder::new(r"(?:^|[^0-9])(([0-2]?\d|3[01])[ _-]+([a-z]{3,10})\.?,?[ _-]+(20\d{2}))(?:[^0-9]|$)").case_insensitive(true).build().expect("valid named date regex")
});
// "Mar 28, 2025", month first as English apps write it.
static MONTH_FIRST: LazyLock<Regex> = LazyLock::new(|| {
    RegexBuilder::new(r"(?:^|[^a-z0-9])(([a-z]{3,10})\.?[ _-]+([0-2]?\d|3[01]),?[ _-]+(20\d{2}))(?:[^0-9]|$)").case_insensitive(true).build().expect("valid month-first regex")
});
// A 13-digit run is a Unix time in milliseconds, as in Facebook's "FB_IMG_1525739935595".
static EPOCH: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?:^|[^0-9])(1[0-9]{12})(?:[^0-9]|$)").expect("valid epoch regex"));
static TIME: LazyLock<Regex> = LazyLock::new(|| {
    RegexBuilder::new(r"^(?:,?\s+at\s+|_at_|,?[-_ ]+)(\d{1,2})[-._]?(\d{2})[-._]?(\d{2})(?:[-.]?\d{1,3})?(?:[_ ]?([ap])\.?\s?m\.?)?").case_insensitive(true).build().expect("valid time regex")
});
static PREFIX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\(\d{2}-\d{2}-\d{2}(?:_\d{2}-\d{2})?\)[_ -]?").expect("valid prefix regex"));

// English, Spanish and French month names; a word counts when it is one of them or its abbreviation, so "Mayhem" is no month.
const MONTHS: [&[&str]; 12] = [
    &["january", "enero", "janvier"], &["february", "febrero", "fevrier"], &["march", "marzo", "mars"], &["april", "abril", "avril"],
    &["may", "mayo", "mai"], &["june", "junio", "juin"], &["july", "julio", "juillet"], &["august", "agosto", "aout"],
    &["september", "septiembre", "setiembre", "septembre"], &["october", "octubre", "octobre"], &["november", "noviembre", "novembre"], &["december", "diciembre", "decembre"],
];

fn month_of(word: &str) -> Option<u32> {
    let word = word.to_lowercase();
    MONTHS.iter().position(|names| names.iter().any(|n| n.starts_with(word.as_str()))).map(|i| i as u32 + 1)
}

fn valid_day(year: u32, month: u32, day: u32) -> bool {
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    (1..=12).contains(&month) && day >= 1 && day <= days[month as usize - 1]
}

fn clock(caps: &Captures) -> Option<(u32, u32)> {
    let (mut hour, minute, second): (u32, u32, u32) = (caps[1].parse().ok()?, caps[2].parse().ok()?, caps[3].parse().ok()?);
    if let Some(m) = caps.get(4) {
        if !(1..=12).contains(&hour) {
            return None;
        }
        hour = hour % 12 + if m.as_str().eq_ignore_ascii_case("p") { 12 } else { 0 };
    }
    (hour <= 23 && minute <= 59 && second <= 59).then_some((hour, minute))
}

// Days since 1970-01-01 to a civil date, by Howard Hinnant's algorithm.
fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(month <= 2), month, day)
}

#[cfg(windows)]
fn to_local(secs: i64) -> i64 {
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::Storage::FileSystem::FileTimeToLocalFileTime;
    const TICKS: i64 = 10_000_000;
    const EPOCH_GAP: i64 = 11_644_473_600;
    let ticks = ((secs + EPOCH_GAP) * TICKS) as u64;
    let utc = FILETIME { dwLowDateTime: ticks as u32, dwHighDateTime: (ticks >> 32) as u32 };
    let mut local = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
    // SAFETY: both pointers are to live FILETIME values on this stack frame.
    if unsafe { FileTimeToLocalFileTime(&utc, &mut local) } == 0 {
        return secs;
    }
    ((u64::from(local.dwHighDateTime) << 32 | u64::from(local.dwLowDateTime)) as i64) / TICKS - EPOCH_GAP
}

#[cfg(not(windows))]
fn to_local(secs: i64) -> i64 {
    secs
}

// The full "yy-mm-dd_hh-mm" of a Unix time, in the user's local time.
pub(super) fn stamp(secs: i64) -> String {
    let local = to_local(secs);
    let (year, month, day) = civil(local.div_euclid(86_400));
    let minutes = local.rem_euclid(86_400) / 60;
    format!("{:02}-{month:02}-{day:02}_{:02}-{:02}", year.rem_euclid(100), minutes / 60, minutes % 60)
}

struct Found {
    start: usize,
    end: usize,
    date: String,
    timed: bool,
}

fn ymd(caps: &Captures, y: usize, m: usize, d: usize) -> Option<(u32, u32, u32)> {
    let year: u32 = caps[y].parse().ok()?;
    let month = caps[m].parse().ok().or_else(|| month_of(&caps[m]))?;
    let day: u32 = caps[d].parse().ok()?;
    valid_day(year, month, day).then_some((year, month, day))
}

fn found(caps: &Captures, date: String, timed: bool) -> Option<Found> {
    let span = caps.get(1)?;
    Some(Found { start: span.start(), end: span.end(), date, timed })
}

fn find(stem: &str) -> Option<Found> {
    let short = |(y, m, d): (u32, u32, u32)| format!("{:02}-{m:02}-{d:02}", y % 100);
    if let Some(c) = COMPACT.captures(stem) {
        if let Some(day) = ymd(&c, 2, 3, 4) {
            return found(&c, format!("{}_{}-{}", short(day), &c[5], &c[6]), true);
        }
    }
    if let Some((c, day)) = DATE.captures(stem).and_then(|c| ymd(&c, 2, 3, 4).map(|d| (c, d))) {
        return found(&c, short(day), false);
    }
    if let Some((c, day)) = NAMED.captures(stem).and_then(|c| ymd(&c, 4, 3, 2).map(|d| (c, d))) {
        return found(&c, short(day), false);
    }
    if let Some((c, day)) = MONTH_FIRST.captures(stem).and_then(|c| ymd(&c, 4, 2, 3).map(|d| (c, d))) {
        return found(&c, short(day), false);
    }
    let c = EPOCH.captures(stem)?;
    let millis: i64 = c[1].parse().ok()?;
    found(&c, stamp(millis / 1000), true)
}

// The date text and the stem without it, or None when the name has no date or already starts with one.
pub(super) fn split_date(stem: &str) -> Option<(String, String)> {
    if PREFIX.is_match(stem) {
        return None;
    }
    let Found { start, end, mut date, timed } = find(stem)?;
    let mut after = &stem[end..];
    if !timed {
        if let Some(time) = TIME.captures(after) {
            let stop = time.get(0).map_or(0, |m| m.end());
            if let Some((hour, minute)) = clock(&time).filter(|_| after[stop..].chars().next().is_none_or(|c| !c.is_ascii_digit())) {
                date.push_str(&format!("_{hour:02}-{minute:02}"));
                after = &after[stop..];
            }
        }
    }
    let seps: &[char] = &['_', '-', ' ', '.', ','];
    let before = stem[..start].trim_matches(seps);
    let tail = after.trim_matches(seps);
    let rest = match (before.is_empty(), tail.is_empty()) {
        (true, _) => tail.to_string(),
        (false, true) => before.to_string(),
        (false, false) => format!("{before}_{tail}"),
    };
    Some((date, rest))
}

// The stem with its "(yy-mm-dd…)_" prefix and any date written in it taken out.
pub(super) fn without_date(stem: &str) -> String {
    let stem = PREFIX.replace(stem, "");
    split_date(&stem).map_or_else(|| stem.into_owned(), |(_, rest)| rest)
}

pub(super) fn has_date(stem: &str) -> bool {
    PREFIX.is_match(stem) || find(stem).is_some()
}
