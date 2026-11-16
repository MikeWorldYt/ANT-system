use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use super::*;
use crate::test_support::TempDir;

const HOLDER_ROOT: &str = "FM_LOCK_HOLDER_ROOT";
const HELD: &str = "LOCK-HELD";

fn lock_path(dir: &TempDir) -> PathBuf {
    data_dir(dir.path()).join(LOCK_FILE)
}

fn write_lock(dir: &TempDir, text: &str, age: Duration) -> PathBuf {
    let path = lock_path(dir);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, text).unwrap();
    age_lock(&path, age);
    path
}

fn age_lock(path: &Path, age: Duration) {
    File::options().write(true).open(path).unwrap().set_modified(SystemTime::now() - age).unwrap();
}

fn message(err: &FmError) -> &str {
    match err {
        FmError::Io { message, .. } => message,
        other => panic!("{other:?}"),
    }
}

fn dead_pid() -> u32 {
    let mut child = Command::new(std::env::current_exe().unwrap()).args(["no-such-test", "--exact"]).stdout(Stdio::null()).spawn().unwrap();
    let pid = child.id();
    child.wait().unwrap();
    pid
}

#[test]
fn storage_lock_is_exclusive_and_released_on_drop() {
    let dir = TempDir::new("lock");
    let held = StorageLock::acquire(dir.path()).unwrap();
    let busy = StorageLock::acquire_within(dir.path(), Duration::ZERO).err().unwrap();
    assert!(is_busy(&busy) && message(&busy).contains("another Folder Manager"), "{busy:?}");
    drop(held);
    drop(StorageLock::acquire(dir.path()).unwrap());
    assert!(!lock_path(&dir).exists());
}

#[test]
fn a_short_lock_is_waited_out() {
    let dir = TempDir::new("lock-wait");
    let held = StorageLock::acquire(dir.path()).unwrap();
    let releaser = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(200));
        drop(held);
    });
    let started = Instant::now();
    drop(StorageLock::acquire_within(dir.path(), Duration::from_secs(5)).unwrap());
    assert!(started.elapsed() >= Duration::from_millis(100));
    releaser.join().unwrap();
}

#[test]
fn threads_never_hold_the_lock_together() {
    let dir = Arc::new(TempDir::new("lock-threads"));
    let inside = Arc::new(AtomicUsize::new(0));
    let workers: Vec<_> = (0..4)
        .map(|_| {
            let (dir, inside) = (dir.clone(), inside.clone());
            std::thread::spawn(move || {
                for _ in 0..5 {
                    let _lock = StorageLock::acquire_within(dir.path(), Duration::from_secs(20)).unwrap();
                    assert_eq!(inside.fetch_add(1, Ordering::SeqCst), 0);
                    std::thread::sleep(Duration::from_millis(10));
                    inside.fetch_sub(1, Ordering::SeqCst);
                }
            })
        })
        .collect();
    workers.into_iter().for_each(|w| w.join().unwrap());
    assert!(!lock_path(&dir).exists());
}

// Run by the test below in a child process: takes the lock and keeps it until it is killed.
#[test]
fn lock_holder_process() {
    let Some(root) = std::env::var_os(HOLDER_ROOT) else { return };
    let _lock = StorageLock::acquire(Path::new(&root)).unwrap();
    println!("{HELD}");
    std::thread::sleep(Duration::from_secs(60));
}

#[test]
fn a_lock_held_by_another_process_survives_refresh_until_its_owner_dies() {
    let dir = TempDir::new("lock-process");
    let holder_test = format!("{}::lock_holder_process", module_path!().split_once("::").unwrap().1);
    let mut holder = Command::new(std::env::current_exe().unwrap())
        .args([holder_test.as_str(), "--exact", "--nocapture"])
        .env(HOLDER_ROOT, dir.path())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let held = BufReader::new(holder.stdout.take().unwrap()).lines().map_while(Result::ok).any(|line| line.contains(HELD));
    assert!(held, "the holder process never took the lock");
    let path = lock_path(&dir);
    age_lock(&path, STALE_LOCK * 2);
    let busy = StorageLock::acquire_within(dir.path(), Duration::ZERO).err().unwrap();
    assert!(is_busy(&busy), "{busy:?}");
    assert!(!clear_stale(dir.path()).unwrap() && path.is_file());
    #[cfg(windows)]
    assert!(fs::remove_file(&path).is_err(), "only the owner may delete a live lock");
    holder.kill().unwrap();
    holder.wait().unwrap();
    let started = Instant::now();
    let stale = loop {
        let err = StorageLock::acquire_within(dir.path(), Duration::ZERO).err().unwrap();
        if !is_busy(&err) || started.elapsed() > Duration::from_secs(5) {
            break err;
        }
        std::thread::sleep(LOCK_POLL);
    };
    assert!(message(&stale).contains("press Refresh") && path.is_file(), "{stale:?}");
    assert!(clear_stale(dir.path()).unwrap() && !path.exists());
    drop(StorageLock::acquire(dir.path()).unwrap());
}

#[test]
fn the_refresh_clears_an_abandoned_lock_and_leaves_a_fresh_one() {
    let dir = TempDir::new("lock-clear");
    let path = write_lock(&dir, &format!("{} token", std::process::id()), Duration::ZERO);
    assert!(!clear_stale(dir.path()).unwrap() && path.exists());
    age_lock(&path, LOCK_WAIT * 2);
    assert!(clear_stale(dir.path()).unwrap() && !path.exists());
    assert!(StorageLock::acquire(dir.path()).is_ok());
}

#[test]
fn a_lock_from_an_older_build_is_judged_by_its_pid_and_age() {
    let dir = TempDir::new("lock-legacy");
    let path = write_lock(&dir, &std::process::id().to_string(), LOCK_WAIT * 2);
    assert!(!clear_stale(dir.path()).unwrap() && path.is_file());
    assert!(is_busy(&StorageLock::acquire_within(dir.path(), Duration::ZERO).err().unwrap()));
    age_lock(&path, STALE_LOCK);
    assert!(clear_stale(dir.path()).unwrap() && !path.exists());
    write_lock(&dir, &dead_pid().to_string(), LOCK_WAIT * 2);
    assert!(clear_stale(dir.path()).unwrap() && !path.exists());
}

#[test]
fn stale_storage_lock_is_reported_and_kept() {
    let dir = TempDir::new("lock-stale");
    let path = write_lock(&dir, "", Duration::from_secs(600));
    let err = StorageLock::acquire(dir.path()).err().unwrap();
    assert!(message(&err).contains("press Refresh"), "{err:?}");
    assert!(path.is_file());
}

#[cfg(not(windows))]
#[test]
fn drop_keeps_a_lock_file_it_no_longer_owns() {
    let dir = TempDir::new("lock-foreign");
    let held = StorageLock::acquire(dir.path()).unwrap();
    fs::write(lock_path(&dir), "1 other").unwrap();
    drop(held);
    assert_eq!(fs::read_to_string(lock_path(&dir)).unwrap(), "1 other");
}
