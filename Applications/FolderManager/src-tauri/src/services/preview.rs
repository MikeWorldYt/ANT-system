// File images for the file icons and the grid: the Windows shell on Windows, and plain image files elsewhere.
use std::path::Path;

use crate::domain::error::R;

pub mod media;

#[cfg(windows)]
mod shell;

#[cfg(not(windows))]
mod external;

#[cfg(windows)]
pub fn file_image(path: &Path, large: bool) -> R<Option<String>> {
    shell::image(path, large)
}

#[cfg(windows)]
pub fn preview_image(path: &Path) -> R<Option<String>> {
    shell::preview(path)
}

#[cfg(not(windows))]
pub fn preview_image(path: &Path) -> R<Option<String>> {
    file_image(path, true)
}

// Images are read as they are; other types go to a platform tool, and the small icons stay generic.
#[cfg(not(windows))]
pub fn file_image(path: &Path, large: bool) -> R<Option<String>> {
    if !large {
        return Ok(None);
    }
    let Some(mime) = image_mime(path) else { return external::thumbnail(path) };
    let bytes = std::fs::read(path).map_err(|e| crate::infra::fs_safe::io_err("read the image", path, &e))?;
    Ok(Some(format!("data:{mime};base64,{}", base64(&bytes))))
}

#[cfg(not(windows))]
fn image_mime(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" | "jfif" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        "bmp" => Some("image/bmp"),
        _ => None,
    }
}

// Pixels are RGBA, row by row, top to bottom; stored deflate blocks keep the encoder small.
#[cfg(any(windows, test))]
pub(crate) fn png_data_url(width: u32, height: u32, rgba: &[u8]) -> String {
    let row = width as usize * 4;
    let mut raw = Vec::with_capacity((row + 1) * height as usize);
    for line in rgba.chunks(row) {
        raw.push(0);
        raw.extend_from_slice(line);
    }
    let mut header = Vec::with_capacity(13);
    header.extend_from_slice(&width.to_be_bytes());
    header.extend_from_slice(&height.to_be_bytes());
    header.extend_from_slice(&[8, 6, 0, 0, 0]);
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    png_chunk(&mut png, b"IHDR", &header);
    png_chunk(&mut png, b"IDAT", &zlib_stored(&raw));
    png_chunk(&mut png, b"IEND", &[]);
    format!("data:image/png;base64,{}", base64(&png))
}

#[cfg(any(windows, test))]
fn png_chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    let mut body = kind.to_vec();
    body.extend_from_slice(data);
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(&body);
    out.extend_from_slice(&crc32(&body).to_be_bytes());
}

#[cfg(any(windows, test))]
fn zlib_stored(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    let blocks: Vec<&[u8]> = data.chunks(65535).collect();
    for (i, block) in blocks.iter().enumerate() {
        out.push(u8::from(i + 1 == blocks.len()));
        let len = block.len() as u16;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(!len).to_le_bytes());
        out.extend_from_slice(block);
    }
    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

#[cfg(any(windows, test))]
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

#[cfg(any(windows, test))]
fn adler32(bytes: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for byte in bytes {
        a = (a + u32::from(*byte)) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

pub(crate) fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk.iter().enumerate().fold(0u32, |acc, (i, b)| acc | u32::from(*b) << (16 - 8 * i));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ALPHABET[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_pads_the_last_group() {
        assert_eq!(base64(b"Man"), "TWFu");
        assert_eq!(base64(b"Ma"), "TWE=");
        assert_eq!(base64(b"M"), "TQ==");
        assert_eq!(base64(b""), "");
    }

    #[test]
    fn checksums_match_the_png_and_zlib_references() {
        assert_eq!(crc32(b"IEND"), 0xAE42_6082);
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
    }

    #[test]
    fn a_png_url_starts_with_the_signature_and_carries_every_pixel_row() {
        let url = png_data_url(2, 2, &[0; 16]);
        let png = base64_decode_prefix(&url, "data:image/png;base64,");
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        assert!(png.windows(4).any(|w| w == b"IEND"));
    }

    fn base64_decode_prefix(url: &str, prefix: &str) -> Vec<u8> {
        assert!(url.starts_with(prefix));
        url[prefix.len()..].bytes().filter(|b| *b != b'=').map(|b| match b {
            b'A'..=b'Z' => b - b'A',
            b'a'..=b'z' => b - b'a' + 26,
            b'0'..=b'9' => b - b'0' + 52,
            b'+' => 62,
            _ => 63,
        })
        .collect::<Vec<u8>>()
        .chunks(4)
        .flat_map(|q| {
            let n = q.iter().enumerate().fold(0u32, |acc, (i, v)| acc | u32::from(*v) << (18 - 6 * i));
            [(n >> 16) as u8, (n >> 8) as u8, n as u8]
        })
        .collect()
    }
}
