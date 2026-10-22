// Rejects folder names that Windows or the storage layout cannot represent, before any disk access.
use crate::error::{FmError, R};

const FORBIDDEN: &[char] = &['\\', '/', ':', '*', '?', '"', '<', '>', '|'];
const MAX_NAME_BYTES: usize = 255;
const RESERVED_DEVICES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL",
    "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
    "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

pub fn validate_component(name: &str) -> R<()> {
    let reject = |reason: &str| FmError::InvalidName {
        name: name.to_string(),
        reason: reason.to_string(),
    };
    if name.is_empty() {
        return Err(reject("empty name"));
    }
    if name == "." || name == ".." {
        return Err(reject("dot segment"));
    }
    if name.chars().any(|c| FORBIDDEN.contains(&c)) {
        return Err(reject("forbidden character"));
    }
    if name.chars().any(char::is_control) {
        return Err(reject("control character"));
    }
    if name.ends_with('.') || name.ends_with(' ') {
        return Err(reject("trailing dot or space"));
    }
    let stem = name.split('.').next().unwrap_or(name);
    if RESERVED_DEVICES.iter().any(|d| d.eq_ignore_ascii_case(stem)) {
        return Err(reject("reserved Windows device name"));
    }
    if name.len() > MAX_NAME_BYTES {
        return Err(reject("longer than 255 bytes"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_ordinary_names() {
        assert!(validate_component("01.Math").is_ok());
        assert!(validate_component("a.b").is_ok());
        assert!(validate_component("COM10").is_ok());
        assert!(validate_component("CONSOLE").is_ok());
        assert!(validate_component("LPT0").is_ok());
        assert!(validate_component(&"a".repeat(255)).is_ok());
    }

    #[test]
    fn rejects_empty_and_dot_segments() {
        assert!(validate_component("").is_err());
        assert!(validate_component(".").is_err());
        assert!(validate_component("..").is_err());
    }

    #[test]
    fn rejects_forbidden_and_control_characters() {
        for name in ["a/b", "a\\b", "a:b", "a*", "a?", "a\"", "a<", "a>", "a|", "a\0b", "a\tb"] {
            assert!(validate_component(name).is_err(), "{name:?} should be rejected");
        }
    }

    #[test]
    fn rejects_trailing_dot_or_space() {
        assert!(validate_component("name.").is_err());
        assert!(validate_component("name ").is_err());
    }

    #[test]
    fn rejects_device_names_in_any_case_and_with_extension() {
        for name in ["CON", "con", "Nul.txt", "COM1", "lpt9.log", "aux.tar.gz"] {
            assert!(validate_component(name).is_err(), "{name:?} should be rejected");
        }
    }

    #[test]
    fn rejects_names_longer_than_255_bytes() {
        assert!(validate_component(&"a".repeat(256)).is_err());
        assert!(validate_component(&"é".repeat(128)).is_err());
    }
}
