use std::fs;
use std::path::Path;

use super::install::{from_staging, stage_repo};
use super::store;
use super::zip_limits::{check, MAX_ENTRIES, MAX_EXPANDED_BYTES};
use crate::test_support::TempDir;

const URL: &str = "https://github.com/owner/repo";
const MANIFEST: &str = r#"{"id":"word-count","name":"Word count","version":"1.2.3","author":"Ann","apiVersion":1,"main":"main.js","permissions":["panel:own"]}"#;
const STORED: u16 = 0;
const DEFLATED: u16 = 8;

struct Entry {
    name: String,
    data: Vec<u8>,
    method: u16,
    expanded: u32,
}

fn stored(name: &str, data: &[u8]) -> Entry {
    Entry { name: name.to_string(), data: data.to_vec(), method: STORED, expanded: data.len() as u32 }
}

struct Bits {
    out: Vec<u8>,
    count: usize,
}

impl Bits {
    fn push(&mut self, bit: u32) {
        if self.count.is_multiple_of(8) {
            self.out.push(0);
        }
        *self.out.last_mut().unwrap() |= ((bit & 1) as u8) << (self.count % 8);
        self.count += 1;
    }

    fn huffman(&mut self, code: u32, width: u32) {
        (0..width).rev().for_each(|i| self.push(code >> i));
    }
}

// One fixed-Huffman block: a zero byte, then repeated length-258 copies at distance 1.
fn deflated_zeros(copies: usize) -> (Vec<u8>, u32) {
    let mut bits = Bits { out: Vec::new(), count: 0 };
    (0..3).for_each(|i| bits.push(0b011 >> i));
    bits.huffman(0b0011_0000, 8);
    for _ in 0..copies {
        bits.huffman(0b1100_0101, 8);
        bits.huffman(0, 5);
    }
    bits.huffman(0, 7);
    (bits.out, 1 + 258 * copies as u32)
}

fn header(out: &mut Vec<u8>, e: &Entry) {
    out.extend(20u16.to_le_bytes());
    out.extend(0u16.to_le_bytes());
    out.extend(e.method.to_le_bytes());
    out.extend([0u8; 8]);
    out.extend((e.data.len() as u32).to_le_bytes());
    out.extend(e.expanded.to_le_bytes());
    out.extend((e.name.len() as u16).to_le_bytes());
    out.extend(0u16.to_le_bytes());
}

fn zip_with_comment(entries: &[Entry], comment: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut directory = Vec::new();
    for e in entries {
        let offset = out.len() as u32;
        out.extend(b"PK\x03\x04");
        header(&mut out, e);
        out.extend(e.name.as_bytes());
        out.extend(&e.data);
        directory.extend(b"PK\x01\x02\x14\0");
        header(&mut directory, e);
        directory.extend([0u8; 10]);
        directory.extend(offset.to_le_bytes());
        directory.extend(e.name.as_bytes());
    }
    let start = out.len() as u32;
    out.extend(&directory);
    out.extend(b"PK\x05\x06\0\0\0\0");
    out.extend((entries.len() as u16).to_le_bytes());
    out.extend((entries.len() as u16).to_le_bytes());
    out.extend((directory.len() as u32).to_le_bytes());
    out.extend(start.to_le_bytes());
    out.extend((comment.len() as u16).to_le_bytes());
    out.extend(comment);
    out
}

fn zip(entries: &[Entry]) -> Vec<u8> {
    zip_with_comment(entries, b"")
}

fn extension(extra: Entry) -> Vec<u8> {
    zip(&[stored("manifest.json", MANIFEST.as_bytes()), stored("main.js", b"ant.panel.show();"), extra])
}

fn nothing_staged(root: &Path) -> bool {
    let staging = store::extensions_dir(root).join(".staging");
    !staging.exists() || fs::read_dir(staging).unwrap().next().is_none()
}

fn preview_token(preview: &super::ExtensionPreview) -> String {
    let value: serde_yaml::Mapping = serde_yaml::from_str(&serde_yaml::to_string(preview).unwrap()).unwrap();
    value["token"].as_str().unwrap().to_string()
}

#[test]
fn a_high_ratio_zip_is_refused_with_nothing_written() {
    let dir = TempDir::new("ext-zip-ratio");
    let (data, expanded) = deflated_zeros(10_000);
    let bytes = extension(Entry { name: "bomb.bin".to_string(), data, method: DEFLATED, expanded });
    let err = stage_repo(dir.path(), URL, &bytes).err().expect("a high-ratio zip must be refused").to_string();
    assert!(err.contains("expands too far"), "{err}");
    assert!(nothing_staged(dir.path()));
}

#[cfg(windows)]
#[test]
fn a_zip_that_understates_its_size_stops_before_expanding() {
    let dir = TempDir::new("ext-zip-lie");
    let (data, expanded) = deflated_zeros((MAX_EXPANDED_BYTES / 258 + 10) as usize);
    assert!(u64::from(expanded) > MAX_EXPANDED_BYTES);
    let declared = data.len() as u32;
    let bytes = extension(Entry { name: "bomb.bin".to_string(), data, method: DEFLATED, expanded: declared });
    let err = stage_repo(dir.path(), URL, &bytes).err().expect("a real size past the limit must be refused").to_string();
    assert!(err.contains("past its limit"), "{err}");
    assert!(nothing_staged(dir.path()));
}

#[test]
fn a_plain_extension_zip_still_stages_and_installs() {
    let dir = TempDir::new("ext-zip-ok");
    let (data, expanded) = deflated_zeros(4);
    let bytes = extension(Entry { name: "docs/zeros.bin".to_string(), data, method: DEFLATED, expanded });
    let token = preview_token(&stage_repo(dir.path(), URL, &bytes).unwrap());
    let info = from_staging(dir.path(), &token).unwrap();
    assert_eq!((info.id.as_str(), info.enabled), ("word-count", false));
    let copied = store::extensions_dir(dir.path()).join("word-count/docs/zeros.bin");
    assert_eq!(fs::metadata(copied).unwrap().len(), u64::from(expanded));
    assert!(nothing_staged(dir.path()));
}

#[test]
fn the_limits_read_the_central_directory() {
    assert!(check(&zip(&[stored("manifest.json", b"{}")])).is_ok());
    assert!(check(&zip_with_comment(&[stored("manifest.json", b"{}")], b"0123456789abcdef")).is_ok());
    assert!(check(&zip(&[])).is_ok());

    let many: Vec<Entry> = (0..=MAX_ENTRIES).map(|i| stored(&format!("{i}.txt"), b"x")).collect();
    assert!(check(&zip(&many)).unwrap_err().to_string().contains("files"));

    let big = Entry { name: "big.bin".to_string(), data: vec![1; 1024 * 1024], method: DEFLATED, expanded: MAX_EXPANDED_BYTES as u32 + 1 };
    assert!(check(&zip(&[big])).unwrap_err().to_string().contains("MB"));
    let marked = Entry { name: "big.bin".to_string(), data: vec![1; 16], method: DEFLATED, expanded: u32::MAX };
    assert!(check(&zip(&[marked])).is_err());
}

#[test]
fn a_directory_that_does_not_add_up_is_refused() {
    let plain = zip(&[stored("manifest.json", b"{}")]);
    assert!(check(b"").is_err());
    assert!(check(b"not a zip at all, long enough to hold an end record").is_err());
    assert!(check(&plain[..plain.len() - 1]).is_err());

    let mut trailing = plain.clone();
    trailing.push(0);
    assert!(check(&trailing).is_err());

    let mut prefixed = b"MZ".to_vec();
    prefixed.extend(&plain);
    assert!(check(&prefixed).is_err());

    let end = plain.len() - 22;
    let mut undercounted = plain.clone();
    undercounted[end + 8] = 0;
    undercounted[end + 10] = 0;
    assert!(check(&undercounted).is_err());
}
