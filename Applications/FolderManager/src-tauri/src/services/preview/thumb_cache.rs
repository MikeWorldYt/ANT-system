// Drawn thumbnails kept on disk in the temp folder, named for the file and its size and date, so a changed file draws anew and a gone one is pruned.
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// Unused entries older than this go even when their file is still there.
const MAX_AGE: Duration = Duration::from_secs(60 * 60 * 24 * 60);

pub fn dir() -> PathBuf {
    std::env::temp_dir().join("ANT").join("thumbs")
}

fn hash(value: impl Hash) -> String {
    let mut h = DefaultHasher::new();
    value.hash(&mut h);
    format!("{:016x}", h.finish())
}

fn stamp(path: &Path) -> Option<String> {
    let meta = fs::metadata(path).ok()?;
    let modified = meta.modified().ok()?.duration_since(UNIX_EPOCH).ok()?.as_nanos();
    Some(hash((meta.len(), modified)))
}

fn source_key(path: &Path) -> String {
    hash(path.to_string_lossy().to_lowercase())
}

// A changed file leaves a stale image under its old stamp; drawing the new one removes it.
fn drop_stale(dir: &Path, prefix: &str, keep: &Path) {
    for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with(prefix) && name.ends_with(".png") && p != keep {
            if let Err(e) = fs::remove_file(&p) {
                eprintln!("Could not remove the old thumbnail {}: {e}", p.display());
            }
        }
    }
}

// The cache is only a speed-up, so a folder that cannot be written still returns the drawn image.
pub fn cached(dir: &Path, path: &Path, pixels: u32, draw: impl FnOnce() -> Option<Vec<u8>>) -> Option<Vec<u8>> {
    let key = source_key(path);
    let Some(stamp) = stamp(path) else { return draw() };
    let prefix = format!("{key}-{pixels}-");
    let file = dir.join(format!("{prefix}{stamp}.png"));
    if let Ok(png) = fs::read(&file) {
        return Some(png);
    }
    let png = draw()?;
    let stored = fs::create_dir_all(dir)
        .and_then(|()| fs::write(&file, &png))
        .and_then(|()| fs::write(dir.join(format!("{key}.src")), path.to_string_lossy().as_bytes()));
    match stored {
        Ok(()) => drop_stale(dir, &prefix, &file),
        Err(e) => eprintln!("Could not cache the thumbnail of {}: {e}", path.display()),
    }
    Some(png)
}

// Each source note names the file its images belong to; once that file is gone, or the images sat unused too long, they all go.
pub fn prune(dir: &Path) {
    let now = SystemTime::now();
    let entries: Vec<PathBuf> = fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).collect();
    for src in entries.iter().filter(|p| p.extension().is_some_and(|e| e == "src")) {
        let Some(key) = src.file_stem().map(|s| s.to_string_lossy().into_owned()) else { continue };
        let gone = fs::read_to_string(src).map(|p| !Path::new(&p).is_file()).unwrap_or(true);
        let images: Vec<&PathBuf> = entries.iter().filter(|p| p.extension().is_some_and(|e| e == "png") && p.file_name().is_some_and(|n| n.to_string_lossy().starts_with(&format!("{key}-")))).collect();
        let old = |p: &Path| fs::metadata(p).and_then(|m| m.modified()).ok().and_then(|t| now.duration_since(t).ok()).is_some_and(|age| age > MAX_AGE);
        for image in images.iter().filter(|p| gone || old(p)) {
            if let Err(e) = fs::remove_file(image) {
                eprintln!("Could not prune the thumbnail {}: {e}", image.display());
            }
        }
        if gone || images.iter().all(|p| !p.exists()) {
            if let Err(e) = fs::remove_file(src) {
                eprintln!("Could not prune {}: {e}", src.display());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    #[test]
    fn a_change_draws_anew_and_a_gone_file_is_pruned() {
        let dir = TempDir::new("thumbs");
        let cache = dir.path().join("cache");
        let doc = dir.path().join("a.pdf");
        fs::write(&doc, "one").unwrap();
        assert_eq!(cached(&cache, &doc, 128, || Some(vec![1])), Some(vec![1]));
        assert_eq!(cached(&cache, &doc, 128, || Some(vec![9])), Some(vec![1]), "the second read comes from disk");
        fs::write(&doc, "changed").unwrap();
        assert_eq!(cached(&cache, &doc, 128, || Some(vec![2])), Some(vec![2]));
        let pngs = || fs::read_dir(&cache).unwrap().flatten().filter(|e| e.path().extension().is_some_and(|x| x == "png")).count();
        assert_eq!(pngs(), 1, "the stale image was dropped");
        fs::remove_file(&doc).unwrap();
        prune(&cache);
        assert_eq!(fs::read_dir(&cache).unwrap().count(), 0);
    }
}
