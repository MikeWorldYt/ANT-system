// Windows scheduled tasks that rerun the app headless (`--backup`, `--sync`, `--watch`) while it is closed.
use std::path::Path;

use crate::domain::error::{FmError, R};
use crate::infra::storage::db::hash;
#[cfg(windows)]
use crate::platform::windows::powershell::run_powershell;
#[cfg(windows)]
use crate::platform::windows::schtasks::{current_user, schtasks};

// Runs as UTF-8 because schtasks prints the code page of the console, which garbles a storage name with accents.
#[cfg(windows)]
const READ_ARGUMENTS: &str = r"[Console]::OutputEncoding = [Text.Encoding]::UTF8; (Get-ScheduledTask -TaskName $env:FM_TASK -TaskPath '\' -ErrorAction Stop).Actions[0].Arguments";

pub struct Task {
    pub name: &'static str,
    pub flag: &'static str,
    pub start: &'static str,
    pub what: &'static str,
}

fn path_key(text: &str) -> String {
    text.replace('/', "\\").trim_end_matches('\\').to_lowercase()
}

// One task per storage, so turning it on in a second storage leaves the first one's alone; a canonical path is no key because a renamed folder has none.
pub fn task_name(task: &Task, root: &Path) -> String {
    format!("{} {}", task.name, hash(&path_key(&root.display().to_string())))
}

fn delete_args(name: &str) -> Vec<String> {
    ["/Delete", "/F", "/TN", name].map(String::from).to_vec()
}

pub fn args(task: &Task, schedule: Option<&str>, exe: &Path, root: &Path) -> Vec<String> {
    let name = task_name(task, root);
    let Some(schedule) = schedule else {
        return delete_args(&name);
    };
    let run = format!("\"{}\" {} \"{}\"", exe.display(), task.flag, root.display());
    ["/Create", "/F", "/SC", schedule, "/TN", &name, "/TR", &run, "/ST", task.start].map(String::from).to_vec()
}

fn runs_on(task: &Task, arguments: &str, root: &Path) -> bool {
    path_key(arguments) == path_key(&format!("{} \"{}\"", task.flag, root.display()))
}

// The new task goes in first, so a failure never leaves the storage with none; a fixed-name task of another storage stays until that storage moves it.
#[cfg(any(windows, test))]
fn take_over(
    task: &Task,
    root: &Path,
    read: impl FnOnce() -> R<Option<String>>,
    create: impl FnOnce() -> R<()>,
    delete: impl FnOnce(&[String]) -> R<()>,
) -> R<()> {
    match read()? {
        Some(arguments) if runs_on(task, &arguments, root) => {
            create()?;
            delete(&delete_args(task.name))
        }
        _ => Ok(()),
    }
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

// schtasks /Create cannot lift the default 72 h execution limit, so a logon task needs its XML.
pub fn logon_xml(task: &Task, user: &str, exe: &Path, root: &Path) -> String {
    let user = escape(user);
    let command = escape(&exe.display().to_string());
    let arguments = escape(&format!("{} \"{}\"", task.flag, root.display()));
    format!(
        r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <Triggers>
    <LogonTrigger>
      <Enabled>true</Enabled>
      <UserId>{user}</UserId>
    </LogonTrigger>
  </Triggers>
  <Principals>
    <Principal id="Author">
      <UserId>{user}</UserId>
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>LeastPrivilege</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <Hidden>true</Hidden>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>{command}</Command>
      <Arguments>{arguments}</Arguments>
    </Exec>
  </Actions>
</Task>
"#
    )
}

// Asking schtasks first keeps PowerShell for the one run that finds the fixed name.
#[cfg(windows)]
fn fixed_name_arguments(task: &Task) -> R<Option<String>> {
    if !schtasks(&["/Query".into(), "/TN".into(), task.name.into()])?.status.success() {
        return Ok(None);
    }
    let env = [("FM_TASK", std::ffi::OsStr::new(task.name))];
    run_powershell(READ_ARGUMENTS, &env, None, std::time::Duration::from_secs(60)).map(Some).map_err(|message| FmError::Internal { message })
}

#[cfg(windows)]
fn delete_fixed_name(task: &Task, args: &[String]) -> R<()> {
    task_result(task, &schtasks(args)?)
}

#[cfg(windows)]
fn drop_fixed_name(task: &Task, root: &Path) -> R<()> {
    take_over(task, root, || fixed_name_arguments(task), || Ok(()), |args| delete_fixed_name(task, args))
}

#[cfg(windows)]
fn write_logon(task: &Task, root: &Path) -> R<()> {
    let exe = app_exe()?;
    let user = current_user()?;
    // Task Scheduler reads a UTF-16 file with a byte order mark.
    let bytes: Vec<u8> = std::iter::once(0xFEFF_u16).chain(logon_xml(task, &user, &exe, root).encode_utf16()).flat_map(u16::to_le_bytes).collect();
    let file = std::env::temp_dir().join(format!("ant-watch-task-{}.xml", std::process::id()));
    std::fs::write(&file, bytes).map_err(|e| FmError::Internal { message: format!("Could not write the task file: {e}") })?;
    let args = ["/Create".to_string(), "/F".into(), "/TN".into(), task_name(task, root), "/XML".into(), file.display().to_string()];
    let out = schtasks(&args);
    if let Err(e) = std::fs::remove_file(&file) {
        eprintln!("Could not remove the task file: {e}");
    }
    task_result(task, &out?)
}

#[cfg(windows)]
pub fn apply_logon(task: &Task, on: bool, root: &Path) -> R<()> {
    if !on {
        return apply(task, None, root);
    }
    write_logon(task, root)?;
    drop_fixed_name(task, root)
}

#[cfg(not(windows))]
pub fn apply_logon(task: &Task, _on: bool, _root: &Path) -> R<()> {
    Err(windows_only(task))
}

#[cfg(windows)]
fn write(task: &Task, schedule: Option<&str>, root: &Path) -> R<()> {
    let exe = app_exe()?;
    let args = args(task, schedule, &exe, root);
    // Querying first makes a missing task a success without parsing localized error text.
    if schedule.is_none() && !schtasks(&["/Query".into(), "/TN".into(), task_name(task, root)])?.status.success() {
        return Ok(());
    }
    task_result(task, &schtasks(&args)?)
}

#[cfg(windows)]
pub fn apply(task: &Task, schedule: Option<&str>, root: &Path) -> R<()> {
    write(task, schedule, root)?;
    drop_fixed_name(task, root)
}

#[cfg(not(windows))]
pub fn apply(task: &Task, _schedule: Option<&str>, _root: &Path) -> R<()> {
    Err(windows_only(task))
}

// Once the fixed name is gone every later run finds nothing to do, so the move happens once.
#[cfg(windows)]
pub fn migrate(task: &Task, schedule: &str, root: &Path) -> R<()> {
    take_over(task, root, || fixed_name_arguments(task), || write(task, Some(schedule), root), |args| delete_fixed_name(task, args))
}

#[cfg(not(windows))]
pub fn migrate(_task: &Task, _schedule: &str, _root: &Path) -> R<()> {
    Ok(())
}

#[cfg(windows)]
pub fn migrate_logon(task: &Task, root: &Path) -> R<()> {
    take_over(task, root, || fixed_name_arguments(task), || write_logon(task, root), |args| delete_fixed_name(task, args))
}

#[cfg(not(windows))]
pub fn migrate_logon(_task: &Task, _root: &Path) -> R<()> {
    Ok(())
}

#[cfg(windows)]
fn app_exe() -> R<std::path::PathBuf> {
    std::env::current_exe().map_err(|e| FmError::Internal { message: format!("Could not find the app: {e}") })
}

#[cfg(windows)]
fn task_result(task: &Task, out: &std::process::Output) -> R<()> {
    if out.status.success() {
        return Ok(());
    }
    let detail = String::from_utf8_lossy(&out.stderr).trim().to_string();
    Err(FmError::Internal { message: format!("Could not update the scheduled {}: {detail}", task.what) })
}

#[cfg(not(windows))]
fn windows_only(task: &Task) -> FmError {
    FmError::Internal { message: format!("Scheduled {} is only available on Windows for now", task.what) }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;

    const TASK: Task = Task { name: "ANT Test", flag: "--backup", start: "12:00", what: "backup" };
    const EXE: &str = "C:/App/fm.exe";

    fn name_in(args: &[String]) -> String {
        let at = args.iter().position(|a| a == "/TN").unwrap();
        args[at + 1].clone()
    }

    #[test]
    fn each_storage_gets_its_own_task_and_turns_off_only_that_one() {
        let one = args(&TASK, Some("DAILY"), Path::new(EXE), Path::new("D:/One"));
        let two = args(&TASK, Some("DAILY"), Path::new(EXE), Path::new("D:/Two"));
        assert_ne!(name_in(&one), name_in(&two));
        assert!(name_in(&one).starts_with("ANT Test "), "{}", name_in(&one));
        assert_eq!(args(&TASK, None, Path::new(EXE), Path::new("D:/One")), ["/Delete", "/F", "/TN", &name_in(&one)]);
    }

    #[test]
    fn two_spellings_of_one_storage_share_a_task() {
        assert_eq!(task_name(&TASK, Path::new("D:/One")), task_name(&TASK, Path::new(r"d:\one\")));
    }

    fn steps(stored: Option<&str>, root: &str, create: R<()>) -> (Vec<String>, R<()>) {
        let log = RefCell::new(Vec::new());
        let result = take_over(
            &TASK,
            Path::new(root),
            || Ok(stored.map(String::from)),
            || {
                log.borrow_mut().push("create".to_string());
                create
            },
            |args| {
                log.borrow_mut().push(args.join(" "));
                Ok(())
            },
        );
        (log.into_inner(), result)
    }

    #[test]
    fn the_fixed_name_of_this_storage_moves_new_task_first() {
        let (log, result) = steps(Some(r#"--backup "D:\One""#), "D:/One", Ok(()));
        assert_eq!(log, ["create", "/Delete /F /TN ANT Test"]);
        assert!(result.is_ok());
    }

    #[test]
    fn with_the_fixed_name_gone_nothing_runs_again() {
        assert_eq!(steps(None, "D:/One", Ok(())).0, Vec::<String>::new());
    }

    #[test]
    fn the_fixed_name_of_another_storage_stays() {
        for stored in [r#"--backup "D:\Two""#, r#"--backup "D:\One Two""#, r#"--sync "D:\One""#] {
            assert_eq!(steps(Some(stored), "D:/One", Ok(())).0, Vec::<String>::new(), "{stored}");
        }
    }

    #[test]
    fn a_failed_new_task_keeps_the_fixed_name() {
        let refused = Err(FmError::Internal { message: "refused".to_string() });
        let (log, result) = steps(Some(r#"--backup "D:\One""#), "D:/One", refused);
        assert_eq!(log, ["create"]);
        assert!(result.is_err());
    }
}
