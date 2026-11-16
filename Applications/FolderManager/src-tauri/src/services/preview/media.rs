// Serves the storage's and Downloads' videos to the side panel preview, with byte ranges so videos can seek.
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use tauri::http::{Request, Response, StatusCode};

use crate::infra::fs_safe::resolve;
use crate::services::downloads::ROOT as DOWNLOADS_ROOT;

pub const SCHEME: &str = "fmmedia";

// Larger ranges are cut, so a seek never reads a whole video into memory.
const MAX_CHUNK: u64 = 4 * 1024 * 1024;

const TYPES: &[(&str, &str)] = &[
    ("png", "image/png"), ("jpg", "image/jpeg"), ("jpeg", "image/jpeg"), ("gif", "image/gif"), ("webp", "image/webp"), ("bmp", "image/bmp"), ("svg", "image/svg+xml"), ("avif", "image/avif"),
    ("mp4", "video/mp4"), ("m4v", "video/mp4"), ("webm", "video/webm"), ("mov", "video/mp4"), ("ogv", "video/ogg"),
];

pub fn mime(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    TYPES.iter().find(|(e, _)| *e == ext).map(|(_, m)| *m)
}

pub fn response(root: Option<&Path>, downloads: Option<&Path>, request: &Request<Vec<u8>>) -> Response<Vec<u8>> {
    serve(root, downloads, request).unwrap_or_else(|status| Response::builder().status(status).body(Vec::new()).unwrap_or_default())
}

fn serve(root: Option<&Path>, downloads: Option<&Path>, request: &Request<Vec<u8>>) -> Result<Response<Vec<u8>>, StatusCode> {
    let rel: Vec<String> = decode(request.uri().path().trim_start_matches('/')).ok_or(StatusCode::BAD_REQUEST)?.split('/').map(str::to_string).collect();
    let (base, rel) = match rel.split_first() {
        Some((first, rest)) if first == DOWNLOADS_ROOT => (downloads, rest),
        _ => (root, &rel[..]),
    };
    let path = resolve(base.ok_or(StatusCode::NOT_FOUND)?, rel).map_err(|_| StatusCode::FORBIDDEN)?;
    let mime = mime(&path).ok_or(StatusCode::UNSUPPORTED_MEDIA_TYPE)?;
    let mut file = File::open(&path).map_err(|_| StatusCode::NOT_FOUND)?;
    let size = file.metadata().map_err(|_| StatusCode::NOT_FOUND)?.len();
    let range = request.headers().get("range").and_then(|v| v.to_str().ok()).and_then(|v| parse_range(v, size));
    let (start, end) = range.unwrap_or((0, size.saturating_sub(1)));
    let end = end.min(start + MAX_CHUNK - 1).min(size.saturating_sub(1));
    let mut body = vec![0; if size == 0 { 0 } else { (end - start + 1) as usize }];
    file.seek(SeekFrom::Start(start)).and_then(|_| file.read_exact(&mut body)).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let builder = Response::builder().header("Content-Type", mime).header("Accept-Ranges", "bytes").header("Content-Length", body.len().to_string());
    let builder = if range.is_some() || body.len() as u64 != size {
        builder.status(StatusCode::PARTIAL_CONTENT).header("Content-Range", format!("bytes {start}-{end}/{size}"))
    } else {
        builder
    };
    builder.body(body).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

fn parse_range(value: &str, size: u64) -> Option<(u64, u64)> {
    let (from, to) = value.strip_prefix("bytes=")?.split(',').next()?.split_once('-')?;
    let (start, end) = match (from.trim(), to.trim()) {
        ("", n) => (size.saturating_sub(n.parse().ok()?), size.checked_sub(1)?),
        (a, "") => (a.parse().ok()?, size.checked_sub(1)?),
        (a, b) => (a.parse().ok()?, b.parse::<u64>().ok()?.min(size.checked_sub(1)?)),
    };
    (start <= end).then_some((start, end))
}

fn decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            out.push(u8::from_str_radix(text.get(i + 1..i + 3)?, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn ranges_and_escapes_are_read() {
        assert_eq!(parse_range("bytes=0-", 10), Some((0, 9)));
        assert_eq!(parse_range("bytes=4-99", 10), Some((4, 9)));
        assert_eq!(parse_range("bytes=-3", 10), Some((7, 9)));
        assert_eq!(parse_range("bytes=8-2", 10), None);
        assert_eq!(decode("100.WORK%2F02.B%2F%C3%B1.png").as_deref(), Some("100.WORK/02.B/ñ.png"));
        assert_eq!(decode("%2"), None);
    }

    proptest! {
        #[test]
        fn a_read_range_always_lies_inside_the_file(value in "bytes=[0-9]{0,7}-[0-9]{0,7}(,[0-9]{0,3}-[0-9]{0,3})?", size in 0u64..1_000_000) {
            if let Some((start, end)) = parse_range(&value, size) {
                prop_assert!(start <= end && end < size, "{} on {} gave {}-{}", value, size, start, end);
            }
        }
    }
}
