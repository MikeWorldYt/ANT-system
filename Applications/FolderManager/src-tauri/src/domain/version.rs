// Orders versions by semver precedence, so the updater can tell a release from its own prereleases.
use std::cmp::Ordering;

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

// Semver 2.0, rule 11: build metadata ignored, a release outranks its prereleases, which compare per identifier.
pub fn compare_versions(a: &str, b: &str) -> Ordering {
    let split = |v: &str| { let v = v.split('+').next().unwrap_or(""); match v.split_once('-') { Some((c, p)) => (parse_version(c), Some(p.to_string())), None => (parse_version(v), None) } };
    let ((ca, pa), (cb, pb)) = (split(a), split(b));
    ca.cmp(&cb).then_with(|| match (pa, pb) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(pa), Some(pb)) => compare_prerelease(&pa, &pb),
    })
}

fn compare_prerelease(a: &str, b: &str) -> Ordering {
    let num = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let (mut ia, mut ib) = (a.split('.'), b.split('.'));
    loop {
        let ord = match (ia.next(), ib.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            // Numbers of any length compare by digit count once leading zeros are gone, then digit by digit.
            (Some(x), Some(y)) if num(x) && num(y) => { let (x, y) = (x.trim_start_matches('0'), y.trim_start_matches('0')); x.len().cmp(&y.len()).then_with(|| x.cmp(y)) }
            (Some(x), Some(_)) if num(x) => Ordering::Less,
            (Some(_), Some(y)) if num(y) => Ordering::Greater,
            (Some(x), Some(y)) => x.cmp(y),
        };
        if ord != Ordering::Equal { return ord; }
    }
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

    #[test]
    fn compare_versions_follows_semver_precedence() {
        use Ordering::*;
        let table = [
            ("2.4.5", "2.4.5-beta", Greater),
            ("2.4.5-beta", "2.4.5", Less),
            ("2.4.5-beta", "2.4.5-beta", Equal),
            ("2.4.5-beta.2", "2.4.5-beta.1", Greater),
            ("2.4.5-beta.10", "2.4.5-beta.9", Greater),
            ("2.4.5-beta", "2.4.5-beta.1", Less),
            ("2.4.5-alpha", "2.4.5-beta", Less),
            ("2.4.5-beta.1", "2.4.5-beta.a", Less),
            ("2.4.5-rc.1", "2.4.5-beta.11", Greater),
            ("2.4.5-1", "2.4.5-alpha", Less),
            ("2.4.5-beta.01", "2.4.5-beta.1", Equal),
            ("2.4.5-beta.99999999999999999999", "2.4.5-beta.9", Greater),
            ("2.4.5+build.7", "2.4.5", Equal),
            ("2.4.5-beta+x", "2.4.5-beta+y", Equal),
            ("2.4.6-beta", "2.4.5", Greater),
            ("1.10.0", "1.9.9", Greater),
            ("1.3.0", "2.0.1-beta", Less),
            // Semver's own example chain from rule 11.
            ("1.0.0-alpha.1", "1.0.0-alpha", Greater),
            ("1.0.0-alpha.beta", "1.0.0-alpha.1", Greater),
            ("1.0.0-beta", "1.0.0-alpha.beta", Greater),
            ("1.0.0-beta.2", "1.0.0-beta", Greater),
            ("1.0.0-beta.11", "1.0.0-beta.2", Greater),
            ("1.0.0-rc.1", "1.0.0-beta.11", Greater),
            ("1.0.0", "1.0.0-rc.1", Greater),
        ];
        for (a, b, want) in table {
            assert_eq!(compare_versions(a, b), want, "{a} vs {b}");
            assert_eq!(compare_versions(b, a), want.reverse(), "{b} vs {a}");
        }
    }
}
