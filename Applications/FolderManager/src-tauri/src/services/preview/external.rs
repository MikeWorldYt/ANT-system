// macOS and Linux have no shell thumbnail API here, so a platform tool draws the thumbnail into a temporary PNG.
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use super::base64;
use crate::error::R;
use crate::fs_safe::io_err;

const PIXELS: &str = "128";
static NEXT_FOLDER: AtomicU64 = AtomicU64::new(0);

// None when the tool is missing or cannot draw this type, which leaves the generic icon in place.
pub fn thumbnail(path: &Path) -> R<Option<String>> {
    let dir = std::env::temp_dir().join(format!("fm-thumb-{}-{}", std::process::id(), NEXT_FOLDER.fetch_add(1, Ordering::Relaxed)));
    std::fs::create_dir_all(&dir).map_err(|e| io_err("create the thumbnail folder", &dir, &e))?;
    let drawn = draw(path, &dir);
    let url = match &drawn {
        Some(png) => Some(format!("data:image/png;base64,{}", base64(&std::fs::read(png).map_err(|e| io_err("read the thumbnail", png, &e))?))),
        None => None,
    };
    // The folder only hands the PNG over, so a leftover one is harmless.
    let _ = std::fs::remove_dir_all(&dir);
    Ok(url)
}

// A tool that is not installed simply draws nothing, so its spawn error is not a failure of the file.
#[cfg(target_os = "macos")]
fn draw(path: &Path, dir: &Path) -> Option<PathBuf> {
    // Quick Look names its thumbnail after the file.
    let out = Command::new("qlmanage").args(["-t", "-s", PIXELS, "-o"]).arg(dir).arg(path).output().ok()?;
    let png = dir.join(format!("{}.png", path.file_name()?.to_string_lossy()));
    (out.status.success() && png.is_file()).then_some(png)
}

#[cfg(not(target_os = "macos"))]
fn draw(path: &Path, dir: &Path) -> Option<PathBuf> {
    let png = dir.join("thumb.png");
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    let out = match ext.as_str() {
        "mp4" | "mkv" | "mov" | "avi" | "webm" | "wmv" | "m4v" => {
            Command::new("ffmpegthumbnailer").arg("-i").arg(path).arg("-o").arg(&png).args(["-s", PIXELS, "-c", "png"]).output()
        }
        "pdf" => Command::new("pdftoppm").args(["-png", "-singlefile", "-scale-to", PIXELS]).arg(path).arg(dir.join("thumb")).output(),
        _ => Command::new("gdk-pixbuf-thumbnailer").args(["-s", PIXELS]).arg(path).arg(&png).output(),
    };
    (out.ok()?.status.success() && png.is_file()).then_some(png)
}
