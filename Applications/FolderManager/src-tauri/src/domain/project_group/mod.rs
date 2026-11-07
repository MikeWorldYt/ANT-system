// Suggests which loose files belong to one project from their names alone; the words of any domain count, and doubtful groups are left for the user to approve.
use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::domain::names::sanitize_train_case;

#[cfg(test)]
mod tests;

const VERSION_WORDS: &[&str] = &[
    "final", "new", "old", "copy", "draft", "version", "ver", "rev", "revised", "revision", "edit", "edited", "export", "exported", "recovered", "backup", "bak", "latest", "last", "wip", "fixed",
    "nuevo", "nueva", "viejo", "copia", "borrador", "ultimo", "ultima", "editado", "revisado", "corregido", "definitivo", "respaldo", "actualizado",
    "the", "and", "for", "of", "de", "la", "el", "los", "las", "del", "con", "para",
];
const NO_OWNER: &[&str] = &["na", "ng", "none", "tbd", "unknown", "recovered"];
// A word or tag in more than this share of the names says nothing about any one project.
const GENERIC_SHARE: f64 = 0.3;
// A small folder may be one project, where every word repeats; the share means little there.
const MIN_NAMES_FOR_GENERIC: usize = 8;
const MIN_TITLE_WORDS_SHARED: usize = 2;
const MIN_OVERLAP: f64 = 0.5;
// Against the fuller name too, so a short name is not pulled in by the few common words it shares with everything.
const MIN_REACH: f64 = 0.35;
const MIN_TYPO_WORD: usize = 5;
const MIN_ID_DIGITS: usize = 4;
const MIN_ID_CODE: usize = 5;
const MAX_TITLE: usize = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Tier {
    Sure,
    Likely,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub title: String,
    pub members: Vec<usize>,
    pub tier: Tier,
    pub owner: Option<String>,
    pub owner_conflict: bool,
    pub created: Option<String>,
}

#[derive(Default)]
struct Parsed {
    ids: BTreeSet<String>,
    keys: BTreeSet<String>,
    words: Vec<String>,
    tags: Vec<String>,
    date: Option<String>,
}

fn stem(name: &str) -> &str {
    match name.rsplit_once('.') {
        Some((head, ext)) if !head.is_empty() && (2..=6).contains(&ext.len()) && ext.chars().all(char::is_alphanumeric) => head,
        _ => name,
    }
}

fn fold(word: &str) -> String {
    word.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'á' | 'à' | 'ä' => 'a',
            'é' | 'è' | 'ë' => 'e',
            'í' | 'ì' | 'ï' => 'i',
            'ó' | 'ò' | 'ö' => 'o',
            'ú' | 'ù' | 'ü' => 'u',
            'ñ' => 'n',
            other => other,
        })
        .collect()
}

fn is_date(text: &str) -> bool {
    let parts: Vec<&str> = text.split('-').collect();
    parts.len() == 3 && parts.iter().all(|p| (2..=4).contains(&p.len()) && p.chars().all(|c| c.is_ascii_digit()))
}

// Dates, copy counters and bracket tags come out first: they are context, never words of the project.
fn strip_marks(text: &str, parsed: &mut Parsed) -> String {
    let mut plain = String::new();
    let mut rest = text;
    while let Some(at) = rest.find(['(', '[']) {
        let close = if rest[at..].starts_with('(') { ')' } else { ']' };
        let Some(len) = rest[at..].find(close) else { break };
        plain.push_str(&rest[..at]);
        plain.push(' ');
        let inner = rest[at + 1..at + len].trim();
        if close == ']' {
            parsed.tags.push(inner.to_string());
        } else if is_date(inner) {
            parsed.date.get_or_insert_with(|| inner.to_string());
        } else if !inner.chars().all(|c| c.is_ascii_digit()) {
            plain.push_str(inner);
        }
        rest = &rest[at + len + 1..];
    }
    plain.push_str(rest);
    plain
}

// Splits on anything but letters and digits, and at camelCase humps.
fn tokens(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut word = String::new();
    let mut lower_before = false;
    for c in text.chars() {
        if !c.is_alphanumeric() {
            lower_before = false;
            if !word.is_empty() {
                out.push(std::mem::take(&mut word));
            }
            continue;
        }
        if c.is_uppercase() && lower_before {
            out.push(std::mem::take(&mut word));
        }
        lower_before = c.is_lowercase();
        word.push(c);
    }
    if !word.is_empty() {
        out.push(word);
    }
    out
}

fn is_year(word: &str) -> bool {
    word.len() == 4 && (word.starts_with("19") || word.starts_with("20"))
}

fn is_version_mark(word: &str) -> bool {
    VERSION_WORDS.contains(&word) || (word.len() > 1 && word.starts_with('v') && word[1..].chars().all(|c| c.is_ascii_digit()))
}

fn is_code(word: &str) -> bool {
    let letters = word.chars().take_while(|c| c.is_ascii_lowercase()).count();
    word.len() >= MIN_ID_CODE && letters > 0 && letters < word.len() && word[letters..].chars().all(|c| c.is_ascii_digit())
}

fn parse(name: &str) -> Parsed {
    let mut parsed = Parsed::default();
    let plain = strip_marks(stem(name), &mut parsed);
    for token in tokens(&plain) {
        let word = fold(&token);
        if word.chars().all(|c| c.is_ascii_digit()) {
            let stamp = word.len() >= 8 && (word.starts_with("19") || word.starts_with("20"));
            if word.len() >= MIN_ID_DIGITS && !is_year(&word) && !stamp {
                parsed.ids.insert(word);
                parsed.words.push(token);
            }
        } else if is_code(&word) {
            parsed.ids.insert(word);
            parsed.words.push(token);
        } else if !is_version_mark(&word) && word.chars().count() >= 2 {
            parsed.keys.insert(word);
            parsed.words.push(token);
        }
    }
    parsed
}

fn one_edit_apart(a: &str, b: &str) -> bool {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    if a.len().abs_diff(b.len()) > 1 {
        return false;
    }
    let same = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    a[(same + 1).min(a.len())..] == b[(same + 1).min(b.len())..] || a[(same + 1).min(a.len())..] == b[same.min(b.len())..] || a[same.min(a.len())..] == b[(same + 1).min(b.len())..]
}

fn same_word(a: &str, b: &str) -> bool {
    a == b || (a.chars().count() >= MIN_TYPO_WORD && b.chars().count() >= MIN_TYPO_WORD && one_edit_apart(a, b))
}

struct Sets(Vec<usize>);

impl Sets {
    fn find(&mut self, i: usize) -> usize {
        if self.0[i] != i {
            let top = self.find(self.0[i]);
            self.0[i] = top;
        }
        self.0[i]
    }

    fn join(&mut self, a: usize, b: usize) {
        let (a, b) = (self.find(a), self.find(b));
        self.0[a] = b;
    }
}

enum Link {
    Strong,
    Doubtful,
}

fn link(a: &Parsed, b: &Parsed, idf: &impl Fn(&str) -> f64) -> Option<Link> {
    if a.ids.intersection(&b.ids).next().is_some() {
        return Some(Link::Strong);
    }
    let anchored = |p: &Parsed| p.keys.len() >= MIN_TITLE_WORDS_SHARED;
    if !anchored(a) || !anchored(b) {
        return None;
    }
    if a.keys == b.keys {
        return Some(Link::Strong);
    }
    let shared: Vec<&String> = a.keys.iter().filter(|x| b.keys.iter().any(|y| same_word(x, y))).collect();
    if shared.len() < MIN_TITLE_WORDS_SHARED {
        return None;
    }
    let weight = |keys: &BTreeSet<String>| keys.iter().map(|k| idf(k)).sum::<f64>();
    let common: f64 = shared.iter().map(|k| idf(k)).sum();
    let (wa, wb) = (weight(&a.keys), weight(&b.keys));
    if common / wa.min(wb).max(f64::MIN_POSITIVE) < MIN_OVERLAP || common / wa.max(wb).max(f64::MIN_POSITIVE) < MIN_REACH {
        return None;
    }
    // Names that differ by one word each may be two projects (Summer, Winter) or one (Intro, Outro), and no rule on names can tell.
    let only_a: Vec<_> = a.keys.difference(&b.keys).collect();
    let only_b: Vec<_> = b.keys.difference(&a.keys).collect();
    if let ([x], [y]) = (only_a.as_slice(), only_b.as_slice()) {
        return Some(if same_word(x, y) { Link::Strong } else { Link::Doubtful });
    }
    Some(if common / wa.max(wb).max(f64::MIN_POSITIVE) >= MIN_OVERLAP { Link::Strong } else { Link::Doubtful })
}

// The fullest name of the group titles it, as it has the most words of the project.
fn title_of(members: &[usize], parsed: &[Parsed]) -> String {
    let best = members.iter().map(|&m| &parsed[m].words).max_by_key(|w| w.len());
    let title = sanitize_train_case(&best.map(|w| w.join(" ")).unwrap_or_default());
    title.chars().take(MAX_TITLE).collect::<String>().trim_end_matches('-').to_string()
}

fn owner_of(members: &[usize], parsed: &[Parsed], shared: &BTreeSet<String>) -> (Option<String>, bool) {
    let mut votes: BTreeMap<String, (usize, String)> = BTreeMap::new();
    for tag in members.iter().flat_map(|&m| &parsed[m].tags) {
        let key = fold(tag);
        let real = key.chars().filter(|c| c.is_alphabetic()).count() >= 2 && !NO_OWNER.contains(&key.as_str()) && !is_version_mark(&key);
        if real && !shared.contains(&key) {
            votes.entry(key).or_insert((0, tag.clone())).0 += 1;
        }
    }
    let conflict = votes.len() > 1;
    if conflict {
        return (None, true);
    }
    (votes.into_values().next().map(|(_, tag)| tag), false)
}

// A tag on many names across several groups is a label of the folder, like a wildcard code, not an owner.
fn shared_tags(groups: &[Vec<usize>], parsed: &[Parsed]) -> BTreeSet<String> {
    let mut files: BTreeMap<String, usize> = BTreeMap::new();
    for tags in parsed.iter().map(|p| p.tags.iter().map(|t| fold(t)).collect::<BTreeSet<_>>()) {
        for tag in tags {
            *files.entry(tag).or_default() += 1;
        }
    }
    files
        .into_iter()
        .filter(|(tag, count)| {
            let reach = groups.iter().filter(|g| g.iter().any(|&m| parsed[m].tags.iter().any(|t| fold(t) == *tag))).count();
            *count as f64 / parsed.len() as f64 > GENERIC_SHARE && reach > 1
        })
        .map(|(tag, _)| tag)
        .collect()
}

pub fn group(names: &[String]) -> Vec<Group> {
    let mut parsed: Vec<Parsed> = names.iter().map(|n| parse(n)).collect();
    let stems: BTreeSet<Vec<String>> = parsed.iter().map(|p| p.keys.iter().cloned().collect()).collect();
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    for key in stems.iter().flatten() {
        *seen.entry(key.clone()).or_default() += 1;
    }
    let total = stems.len().max(1) as f64;
    if stems.len() >= MIN_NAMES_FOR_GENERIC {
        for p in &mut parsed {
            p.keys.retain(|k| seen.get(k).copied().unwrap_or(0) as f64 / total <= GENERIC_SHARE);
        }
    }
    let idf = |key: &str| (total / seen.get(key).copied().unwrap_or(1) as f64).ln().max(0.1);

    let mut sets = Sets((0..names.len()).collect());
    let mut doubtful = Vec::new();
    for i in 0..names.len() {
        for j in i + 1..names.len() {
            match link(&parsed[i], &parsed[j], &idf) {
                Some(Link::Strong) => sets.join(i, j),
                Some(Link::Doubtful) => doubtful.push((i, j)),
                None => {}
            }
        }
    }
    let firm: Vec<usize> = (0..names.len()).map(|i| sets.find(i)).collect();
    for (i, j) in doubtful.iter().copied() {
        sets.join(i, j);
    }
    let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for i in 0..names.len() {
        groups.entry(sets.find(i)).or_default().push(i);
    }
    let found: Vec<Vec<usize>> = groups.into_values().filter(|g| g.len() > 1).collect();
    let shared = shared_tags(&found, &parsed);
    found
        .into_iter()
        .map(|members| {
            let certain = members.iter().all(|&m| firm[m] == firm[members[0]]);
            let (owner, owner_conflict) = owner_of(&members, &parsed, &shared);
            let created = members.iter().filter_map(|&m| parsed[m].date.clone()).min();
            Group { title: title_of(&members, &parsed), tier: if certain { Tier::Sure } else { Tier::Likely }, members, owner, owner_conflict, created }
        })
        .collect()
}
