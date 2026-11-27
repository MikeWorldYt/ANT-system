// Finds the words several file names share, as the name-files scanner script did, so the agent starts from real candidates instead of reading every name.
use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

use regex::Regex;

const EXAMPLES: usize = 3;
// A huge folder would flood the prompt; the most shared words matter most.
const MAX_CANDIDATES: usize = 600;

static GROUPS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[(.*?)\]|\((.*?)\)").expect("bracket regex"));
static COUNTER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"-\(\d+\)").expect("counter regex"));
static DASHES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"-{2,}").expect("dashes regex"));
static TAIL_NUMBER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"-\d+$").expect("tail regex"));
static DATE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\d{2,4}-\d{2}-\d{2}$").expect("date regex"));
static HEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[a-f0-9]{5,}$").expect("hex regex"));

#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub token: String,
    pub count: usize,
    pub examples: Vec<String>,
}

fn stem(name: &str) -> &str {
    match name.rfind('.') {
        Some(at) if at > 0 => &name[..at],
        _ => name,
    }
}

fn is_number(word: &str) -> bool {
    !word.is_empty() && word.chars().all(char::is_numeric)
}

// "[Artist Name]" and "(Some Site)" usually name a subject on their own, so each is a token as written, dashed.
fn bracketed(stem: &str) -> Vec<String> {
    GROUPS
        .captures_iter(stem)
        .filter_map(|c| c.get(1).or_else(|| c.get(2)).map(|m| m.as_str()))
        .filter(|inner| !is_number(inner))
        .map(|inner| inner.trim().to_lowercase().replace([' ', '_'], "-"))
        .filter(|token| token.chars().count() > 1)
        .collect()
}

fn dashed(stem: &str) -> String {
    let base = GROUPS.replace_all(stem, "").to_lowercase().replace([' ', '_', '.'], "-");
    let base = COUNTER.replace_all(&base, "");
    DASHES.replace_all(&base, "-").into_owned()
}

pub(super) fn normalize(stem: &str) -> String {
    TAIL_NUMBER.replace(&dashed(stem), "").trim_matches('-').to_string()
}

pub(super) fn tokens(name: &str) -> BTreeSet<String> {
    let stem = stem(name);
    let mut out: BTreeSet<String> = bracketed(stem).into_iter().collect();
    let base = normalize(stem);
    // Checked before the trailing number goes, which would cut "2024-05-01" to "2024-05".
    if base.is_empty() || DATE.is_match(dashed(stem).trim_matches('-')) {
        return out;
    }
    out.extend(base.split('-').filter(|w| w.chars().count() > 1 && !is_number(w) && !HEX.is_match(w)).map(str::to_string));
    out.insert(base);
    out
}

// Each file counts once per token, so a word repeated inside one name never passes for a shared one.
pub fn shared(names: &[String]) -> Vec<Candidate> {
    let mut map: BTreeMap<String, Vec<&String>> = BTreeMap::new();
    for name in names {
        for token in tokens(name) {
            map.entry(token).or_default().push(name);
        }
    }
    let mut found: Vec<Candidate> = map
        .into_iter()
        .filter(|(_, files)| files.len() > 1)
        .map(|(token, files)| Candidate { token, count: files.len(), examples: files.iter().take(EXAMPLES).map(|f| f.to_string()).collect() })
        .collect();
    if found.len() > MAX_CANDIDATES {
        found.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.token.cmp(&b.token)));
        found.truncate(MAX_CANDIDATES);
        found.sort_by(|a, b| a.token.cmp(&b.token));
    }
    found
}

pub fn render(title: &str, files: usize, found: &[Candidate]) -> String {
    let mut out = format!("# Shared words in the file names of {title}\n\n{files} files scanned, {} words shared by two or more.\n\n", found.len());
    for c in found {
        out.push_str(&format!("- {} ({}): {}\n", c.token, c.count, c.examples.join(" | ")));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn dictionary_scan_normalizes_names_like_the_script() {
        assert_eq!(normalize("My_Cool.Photo  Shot-(2)"), "my-cool-photo-shot");
        assert_eq!(normalize("Report__final--v-12"), "report-final-v");
        assert_eq!(normalize("[Artist] Blue Archive (Fanart)"), "blue-archive");
        assert_eq!(normalize("-trimmed-"), "trimmed");
    }

    #[test]
    fn dictionary_scan_keeps_words_and_drops_numbers_hashes_and_dates() {
        let found = tokens("[Some Artist] Blue_Archive-a1b2c3d4 (12) 2024.png");
        assert!(found.contains("some-artist") && found.contains("blue") && found.contains("archive"), "{found:?}");
        assert!(!found.iter().any(|t| t == "12" || t == "2024" || t == "a1b2c3d4"), "{found:?}");
        assert!(tokens("2024-05-01.jpg").is_empty());
        assert_eq!(tokens("[Fan Club] 2024-05-01.jpg").into_iter().collect::<Vec<_>>(), ["fan-club"]);
    }

    #[test]
    fn dictionary_scan_keeps_only_words_shared_by_two_files() {
        let found = shared(&names(&["Blue Archive 01.png", "blue_archive 02.png", "Touhou Reimu.png", "touhou touhou.jpg"]));
        let words: Vec<(&str, usize)> = found.iter().map(|c| (c.token.as_str(), c.count)).collect();
        assert_eq!(words, [("archive", 2), ("blue", 2), ("blue-archive", 2), ("touhou", 2)]);
        assert_eq!(found[0].examples, ["Blue Archive 01.png", "blue_archive 02.png"]);
        assert!(render("Downloads", 4, &found).contains("- touhou (2): Touhou Reimu.png | touhou touhou.jpg"));
    }
}
