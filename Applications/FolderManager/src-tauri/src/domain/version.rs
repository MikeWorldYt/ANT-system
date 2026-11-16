// Orders versions by their numeric parts.

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

#[cfg(test)]
mod tests {
    use super::*;

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
