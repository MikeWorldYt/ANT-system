// Reads the app version PyInstaller stamps into version.txt, and orders versions numerically.
const FILE_VERSION_HEAD: &str = "StringStruct('FileVersion',";

pub fn parse_file_version(txt: &str) -> Option<String> {
    let mut rest = txt;
    while let Some(pos) = rest.find(FILE_VERSION_HEAD) {
        let after = &rest[pos + FILE_VERSION_HEAD.len()..];
        if let Some(version) = quoted_version(after.trim_start()) {
            return Some(format!("v{version}"));
        }
        rest = after;
    }
    None
}

pub fn parse_version(s: &str) -> [u32; 3] {
    let mut out = [0u32; 3];
    for (slot, run) in out
        .iter_mut()
        .zip(s.split(|c: char| !c.is_ascii_digit()).filter(|p| !p.is_empty()))
    {
        *slot = run.parse().unwrap_or(u32::MAX);
    }
    out
}

fn quoted_version(s: &str) -> Option<&str> {
    let inner = s.strip_prefix('\'')?;
    let run = inner
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .unwrap_or(inner.len());
    if run == 0 {
        return None;
    }
    let (version, tail) = inner.split_at(run);
    if !tail.starts_with("')") {
        return None;
    }
    Some(version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_file_version_reads_stamped_version() {
        let txt = "VSVersionInfo(StringStruct('CompanyName', 'ANT'), StringStruct('FileVersion', '1.4.2'))";
        assert_eq!(parse_file_version(txt), Some("v1.4.2".to_string()));
        assert_eq!(
            parse_file_version("StringStruct('FileVersion',\t'2.0')"),
            Some("v2.0".to_string())
        );
    }

    #[test]
    fn parse_file_version_rejects_unmatched_text() {
        assert_eq!(parse_file_version("StringStruct('FileVersion', 'x')"), None);
        assert_eq!(parse_file_version("StringStruct('FileVersion', '1.2'"), None);
        assert_eq!(parse_file_version(""), None);
    }

    #[test]
    fn parse_version_takes_first_three_numbers() {
        assert_eq!(parse_version("v1.2.3-beta"), [1, 2, 3]);
        assert_eq!(parse_version("2"), [2, 0, 0]);
        assert_eq!(parse_version(""), [0, 0, 0]);
        assert_eq!(parse_version("1.2.3.4"), [1, 2, 3]);
    }

    #[test]
    fn parse_version_compares_numerically() {
        assert!(parse_version("1.10.0") > parse_version("1.9.9"));
    }
}
