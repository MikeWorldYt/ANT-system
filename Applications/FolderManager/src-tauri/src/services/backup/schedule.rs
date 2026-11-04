// When an automatic backup is due, and the Windows scheduled task that runs it while the app is closed.
use std::path::Path;

use crate::error::{FmError, R};
use crate::scheduler::{self, Task};

const TASK_NAME: &str = "ANT System Manager Backup";
const TASK: Task = Task { name: TASK_NAME, flag: "--backup", start: "12:00", what: "backup" };
const DAY: u64 = 86_400;

pub fn period_secs(frequency: &str) -> Option<u64> {
    match frequency {
        "day" => Some(DAY),
        "week" => Some(7 * DAY),
        "month" => Some(30 * DAY),
        _ => None,
    }
}

// A clock set backwards makes `last` lie in the future, which waits instead of backing up on every launch.
pub fn is_due(auto: bool, frequency: &str, last: Option<u64>, now: u64) -> bool {
    match (auto, period_secs(frequency), last) {
        (true, Some(_), None) => true,
        (true, Some(period), Some(last)) => now.saturating_sub(last) >= period,
        _ => false,
    }
}

fn schedule(on: bool, frequency: &str) -> R<Option<&'static str>> {
    if !on {
        return Ok(None);
    }
    match frequency {
        "day" => Ok(Some("DAILY")),
        "week" => Ok(Some("WEEKLY")),
        "month" => Ok(Some("MONTHLY")),
        _ => Err(FmError::Internal { message: format!("Unknown backup frequency {frequency}") }),
    }
}

pub fn apply(on: bool, frequency: &str, root: &Path) -> R<()> {
    scheduler::apply(&TASK, schedule(on, frequency)?, root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_backup_is_due_once_the_period_has_passed() {
        assert!(is_due(true, "day", None, 100));
        assert!(!is_due(false, "day", None, 100));
        assert!(!is_due(true, "year", None, 100));
        assert!(!is_due(true, "day", Some(1000), 1000 + DAY - 1));
        assert!(is_due(true, "day", Some(1000), 1000 + DAY));
        assert!(!is_due(true, "week", Some(0), 6 * DAY));
        assert!(is_due(true, "month", Some(0), 30 * DAY));
        assert!(!is_due(true, "day", Some(5000), 10));
    }

    fn schtasks_args(on: bool, frequency: &str, exe: &Path, root: &Path) -> R<Vec<String>> {
        Ok(scheduler::args(&TASK, schedule(on, frequency)?, exe, root))
    }

    #[test]
    fn schtasks_arguments_follow_the_frequency() {
        let args = schtasks_args(true, "week", Path::new("C:/App/fm.exe"), Path::new("D:/My Storage")).unwrap();
        let expected = ["/Create", "/F", "/SC", "WEEKLY", "/TN", TASK_NAME, "/TR"];
        assert_eq!(args[..7], expected);
        assert_eq!(args[7], "\"C:/App/fm.exe\" --backup \"D:/My Storage\"");
        assert_eq!(args[8..], ["/ST", "12:00"]);
        assert_eq!(schtasks_args(true, "day", Path::new("a"), Path::new("b")).unwrap()[3], "DAILY");
        assert_eq!(schtasks_args(true, "month", Path::new("a"), Path::new("b")).unwrap()[3], "MONTHLY");
        assert!(schtasks_args(true, "hour", Path::new("a"), Path::new("b")).is_err());
        assert_eq!(schtasks_args(false, "", Path::new("a"), Path::new("b")).unwrap(), ["/Delete", "/F", "/TN", TASK_NAME]);
    }
}
