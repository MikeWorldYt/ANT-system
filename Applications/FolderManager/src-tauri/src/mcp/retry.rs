// The storage lock waits 3 s; a change in the app that takes longer is ridden out with a few spaced tries, then reported.
use std::thread::sleep;
use std::time::Duration;

use crate::domain::error::{FmError, R};
use crate::infra::fs_safe::is_busy;

#[derive(Clone, Copy)]
pub struct Retry {
    pub attempts: u32,
    pub backoff: Duration,
}

pub const RETRY: Retry = Retry { attempts: 3, backoff: Duration::from_millis(250) };

// Each try takes and drops the lock inside the mutation, so it is never held between tries or tool calls.
pub fn retrying<T>(retry: Retry, f: impl Fn() -> R<T>) -> R<T> {
    let attempts = retry.attempts.max(1);
    let mut pause = retry.backoff;
    for _ in 1..attempts {
        match f() {
            Err(e) if is_busy(&e) => {
                sleep(pause);
                pause *= 2;
            }
            other => return other,
        }
    }
    f().map_err(|e| if is_busy(&e) { gave_up(e, attempts) } else { e })
}

// The lock is taken before anything is touched, so a refusal here never leaves a half-done change.
fn gave_up(e: FmError, attempts: u32) -> FmError {
    let FmError::Io { op, path, os_code, .. } = e else { return e };
    let message = format!(
        "Folder Manager or another process kept this storage locked through {attempts} tries; nothing was changed, call the tool again in a moment"
    );
    FmError::Io { op, path, message, os_code }
}
