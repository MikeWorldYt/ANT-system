// A frame of a video drawn by the bundled ffmpeg, which decodes HEVC and others without the codecs Windows may lack.
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::platform::windows::powershell::CREATE_NO_WINDOW;

const VIDEO: &[&str] = &["mp4", "m4v", "mov", "mkv", "avi", "webm", "wmv", "3gp"];
// A second in skips the black first frame most clips open on; a shorter clip falls back to its start.
const SEEKS: [&str; 2] = ["1", "0"];

pub fn is_video(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| VIDEO.contains(&e.to_ascii_lowercase().as_str()))
}

// Tauri places the bundled binary beside the app's own executable.
fn ffmpeg() -> Option<PathBuf> {
    let path = std::env::current_exe().ok()?.parent()?.join("ffmpeg.exe");
    path.is_file().then_some(path)
}

// None when ffmpeg is missing or cannot read the file, which leaves the shell's image in place.
pub fn frame(path: &Path, pixels: u32) -> Option<Vec<u8>> {
    let exe = ffmpeg()?;
    let scale = format!("scale='min({pixels},iw)':-2");
    SEEKS.iter().find_map(|seek| {
        let out = Command::new(&exe)
            .args(["-hide_banner", "-loglevel", "error", "-ss", seek, "-i"])
            .arg(path)
            .args(["-frames:v", "1", "-vf", &scale, "-f", "image2pipe", "-vcodec", "png", "-"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .ok()?;
        (out.status.success() && !out.stdout.is_empty()).then_some(out.stdout)
    })
}
