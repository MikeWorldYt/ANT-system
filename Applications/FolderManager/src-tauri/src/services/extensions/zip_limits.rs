// Reads a zip's central directory, so an archive that expands too far is refused before anything reaches the disk.
use super::fail;
use crate::domain::error::R;

pub const MAX_EXPANDED_BYTES: u64 = 50 * 1024 * 1024;
pub const MAX_ENTRIES: u64 = 5_000;
const MAX_RATIO: u64 = 100;

const END_SIGNATURE: &[u8] = b"PK\x05\x06";
const END_LEN: usize = 22;
const ENTRY_SIGNATURE: &[u8] = b"PK\x01\x02";
const ENTRY_LEN: usize = 46;

struct Sizes {
    compressed: u64,
    expanded: u64,
}

fn number<const N: usize>(zip: &[u8], at: usize) -> Option<u64> {
    let bytes: [u8; N] = zip.get(at..at.checked_add(N)?)?.try_into().ok()?;
    Some(bytes.iter().rev().fold(0, |n, b| (n << 8) | u64::from(*b)))
}

fn sizes(zip: &[u8], end: usize, start: usize, count: u64) -> Option<Sizes> {
    let mut at = start;
    let mut total = Sizes { compressed: 0, expanded: 0 };
    for _ in 0..count {
        if zip.get(at..at + ENTRY_SIGNATURE.len())? != ENTRY_SIGNATURE {
            return None;
        }
        total.compressed += number::<4>(zip, at + 20)?;
        total.expanded += number::<4>(zip, at + 24)?;
        let names = number::<2>(zip, at + 28)? + number::<2>(zip, at + 30)? + number::<2>(zip, at + 32)?;
        at += ENTRY_LEN + names as usize;
    }
    (at == end).then_some(total)
}

// Every field must add up, because a reader that disagrees with .NET about where the directory sits would check another file list.
fn directory(zip: &[u8]) -> Option<(u64, usize, usize)> {
    let end = zip.windows(END_SIGNATURE.len()).rposition(|w| w == END_SIGNATURE)?;
    let count = number::<2>(zip, end + 10)?;
    let start = number::<4>(zip, end + 16)? as usize;
    let fits = end + END_LEN + number::<2>(zip, end + 20)? as usize == zip.len()
        && number::<2>(zip, end + 8)? == count
        && start + number::<4>(zip, end + 12)? as usize == end;
    fits.then_some((count, start, end))
}

pub fn check(zip: &[u8]) -> R<()> {
    let unreadable = || fail("The archive is not a plain zip".to_string());
    let (count, start, end) = directory(zip).ok_or_else(unreadable)?;
    if count > MAX_ENTRIES {
        return Err(fail(format!("The archive holds more than {MAX_ENTRIES} files")));
    }
    let total = sizes(zip, end, start, count).ok_or_else(unreadable)?;
    if total.expanded > MAX_EXPANDED_BYTES {
        return Err(fail(format!("The archive expands past {} MB", MAX_EXPANDED_BYTES / (1024 * 1024))));
    }
    if total.expanded > total.compressed.saturating_mul(MAX_RATIO) {
        return Err(fail("The archive expands too far for its size".to_string()));
    }
    Ok(())
}
