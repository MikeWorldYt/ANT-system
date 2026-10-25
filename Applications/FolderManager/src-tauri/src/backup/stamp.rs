// Local-time stamp for backups started without the frontend, which normally supplies it.
pub fn format_stamp(year: u32, month: u32, day: u32, hour: u32, minute: u32) -> String {
    format!("{:02}-{month:02}-{day:02}-{hour:02}-{minute:02}", year % 100)
}

#[cfg(windows)]
#[repr(C)]
struct SystemTime {
    year: u16,
    month: u16,
    day_of_week: u16,
    day: u16,
    hour: u16,
    minute: u16,
    second: u16,
    milliseconds: u16,
}

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn GetLocalTime(time: *mut SystemTime);
}

#[cfg(windows)]
pub fn local_stamp() -> String {
    let mut t = SystemTime { year: 0, month: 0, day_of_week: 0, day: 0, hour: 0, minute: 0, second: 0, milliseconds: 0 };
    // SAFETY: GetLocalTime only fills the caller-owned struct and cannot fail.
    unsafe { GetLocalTime(&mut t) };
    format_stamp(t.year.into(), t.month.into(), t.day.into(), t.hour.into(), t.minute.into())
}

// Without a time-zone API in std this stamp is UTC, not local time.
#[cfg(not(windows))]
pub fn local_stamp() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let (days, rest) = ((secs / 86_400) as i64, secs % 86_400);
    // Days-to-civil conversion from Howard Hinnant's date algorithms.
    let z = days + 719_468;
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + z.div_euclid(146_097) * 400 + i64::from(month <= 2);
    format_stamp(year as u32, month as u32, day as u32, (rest / 3600) as u32, (rest % 3600 / 60) as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::debug_log::valid_stamp;

    #[test]
    fn stamps_use_two_digit_groups() {
        assert_eq!(format_stamp(2026, 1, 5, 9, 3), "26-01-05-09-03");
        assert_eq!(format_stamp(2100, 12, 31, 23, 59), "00-12-31-23-59");
    }

    #[test]
    fn the_local_stamp_is_a_valid_stamp() {
        assert!(valid_stamp(&local_stamp()));
    }
}
