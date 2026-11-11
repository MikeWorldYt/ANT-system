// Finds which dictionary entry a file name belongs to, by words, extension or pattern, and tidies the name inside its folder.
use std::collections::HashSet;

use super::dictionary::Entry;

const SHORT_TERM: usize = 4;
const SEPARATORS: &[char] = &[' ', '_', '-', '.'];

// One char in, one char out, so spans found on the folded text stay valid on the name.
fn fold(c: char) -> char {
    let lower = c.to_lowercase().next().unwrap_or(c);
    match lower {
        'á' | 'à' | 'ä' | 'â' | 'ã' => 'a',
        'é' | 'è' | 'ë' | 'ê' => 'e',
        'í' | 'ì' | 'ï' | 'î' => 'i',
        'ó' | 'ò' | 'ö' | 'ô' | 'õ' => 'o',
        'ú' | 'ù' | 'ü' | 'û' => 'u',
        'ñ' => 'n',
        'ç' => 'c',
        other => other,
    }
}

struct Token {
    text: String,
    start: usize,
    end: usize,
}

// Runs of letters and digits; spans are byte offsets.
fn coarse(text: &str) -> Vec<Token> {
    let mut out: Vec<Token> = Vec::new();
    let mut open: Option<usize> = None;
    for (i, c) in text.char_indices().chain(std::iter::once((text.len(), ' '))) {
        match (c.is_alphanumeric(), open) {
            (true, None) => open = Some(i),
            (false, Some(start)) => {
                out.push(Token { text: text[start..i].chars().map(fold).collect(), start, end: i });
                open = None;
            }
            _ => {}
        }
    }
    out
}

#[derive(PartialEq, Clone, Copy)]
enum Kind {
    Upper,
    Lower,
    Digit,
    Other,
}

fn kind(c: char) -> Kind {
    if c.is_ascii_digit() {
        Kind::Digit
    } else if c.is_uppercase() {
        Kind::Upper
    } else if c.is_lowercase() {
        Kind::Lower
    } else {
        Kind::Other
    }
}

// "BlueArchive" reads as blue + archive and "HTMLPage" as html + page, as the words a name joins.
fn fine(text: &str) -> Vec<Token> {
    let mut out = Vec::new();
    for word in coarse(text) {
        let chars: Vec<(usize, char)> = text[word.start..word.end].char_indices().map(|(i, c)| (i + word.start, c)).collect();
        let mut start = 0;
        for i in 1..=chars.len() {
            let split = i == chars.len() || {
                let (a, b) = (kind(chars[i - 1].1), kind(chars[i].1));
                let next_lower = chars.get(i + 1).is_some_and(|&(_, c)| kind(c) == Kind::Lower);
                (a != b && !(a == Kind::Upper && b == Kind::Lower)) || (a == Kind::Upper && b == Kind::Upper && next_lower)
            };
            if split {
                let (s, e) = (chars[start].0, chars.get(i).map_or(word.end, |&(p, _)| p));
                out.push(Token { text: text[s..e].chars().map(fold).collect(), start: s, end: e });
                start = i;
            }
        }
    }
    out
}

fn find_run(tokens: &[Token], key: &[String]) -> Vec<(usize, usize)> {
    if key.is_empty() || key.len() > tokens.len() {
        return Vec::new();
    }
    tokens.windows(key.len()).filter(|w| w.iter().zip(key).all(|(t, k)| t.text == *k)).map(|w| (w[0].start, w[w.len() - 1].end)).collect()
}

// A short term hides inside hashes ("ba3f"), so it only counts as a whole word or written in capitals.
fn short_ok(stem: &str, (s, e): (usize, usize), words: &[Token]) -> bool {
    words.iter().any(|w| w.start == s && w.end == e) || stem[s..e].chars().all(|c| !c.is_lowercase())
}

// `only` is literal: case and symbols must match as written, and only as a whole word.
fn find_strict(stem: &str, term: &str) -> Option<(usize, usize)> {
    let free = |c: Option<char>| c.is_none_or(|c| !c.is_alphanumeric());
    stem.match_indices(term).map(|(s, _)| (s, s + term.len())).find(|&(s, e)| free(stem[..s].chars().next_back()) && free(stem[e..].chars().next()))
}

pub fn find_term(stem: &str, term: &str) -> Option<(usize, usize)> {
    let words = coarse(stem);
    let parts = fine(stem);
    let split: Vec<String> = fine(term).into_iter().map(|t| t.text).collect();
    let joined: Vec<String> = coarse(term).into_iter().map(|t| t.text).collect();
    let short = split.iter().map(|t| t.chars().count()).sum::<usize>() < SHORT_TERM;
    find_run(&parts, &split).into_iter().chain(find_run(&words, &joined)).find(|&span| !short || short_ok(stem, span, &words))
}

pub fn split_ext(name: &str) -> (&str, &str) {
    match name.rfind('.') {
        Some(dot) if dot > 0 => (&name[..dot], &name[dot..]),
        _ => (name, ""),
    }
}

fn longest<'a>(stem: &str, terms: &'a [String], find: fn(&str, &str) -> Option<(usize, usize)>) -> Option<usize> {
    terms.iter().filter(|t| find(stem, t).is_some()).map(|t| t.len()).max()
}

// The longest matched term wins, so "Genshin Impact" beats a stray "ba"; an extension or pattern only decides when no term matched.
pub fn classify<'a>(entries: &'a [Entry], file: &str) -> Option<&'a Entry> {
    let (stem, ext) = split_ext(file);
    let ext = ext.to_lowercase();
    let by_term = entries
        .iter()
        .filter_map(|e| [longest(stem, &e.terms, find_term), longest(stem, &e.topics, find_term), longest(stem, &e.only, find_strict)].into_iter().flatten().max().map(|len| (e, len)))
        .max_by_key(|&(_, len)| len)
        .map(|(e, _)| e);
    by_term.or_else(|| entries.iter().find(|e| e.exts.contains(&ext) || e.patterns.iter().any(|p| p.is_match(stem))))
}

// Drops a span with the separators around it, joining what is left with the separator it had, as in "WAL_sensei-come" to "WAL_come".
fn drop_span(stem: &str, s: usize, e: usize) -> Option<String> {
    let left = stem[..s].trim_end_matches(SEPARATORS);
    let right = stem[e..].trim_start_matches(SEPARATORS);
    let joint = stem[left.len()..s].chars().next().or_else(|| stem[e..].chars().next().filter(|c| SEPARATORS.contains(c))).unwrap_or(' ');
    match (left.is_empty(), right.is_empty()) {
        (true, true) => None,
        (true, false) => Some(right.to_string()),
        (false, true) => Some(left.to_string()),
        (false, false) => Some(format!("{left}{joint}{right}")),
    }
}

// Every occurrence goes; a cut that would leave the name empty keeps it whole.
fn cut_all(name: &str, span: impl Fn(&str) -> Option<(usize, usize)>) -> String {
    let mut name = name.to_string();
    loop {
        let (stem, ext) = split_ext(&name);
        let Some(rest) = span(stem).and_then(|(s, e)| drop_span(stem, s, e)) else { return name };
        name = format!("{rest}{ext}");
    }
}

// Each word of the topics stays where it first shows and leaves where it repeats: "mika-misono-mika-ixss" reads "mika-misono-ixss".
fn dedupe_topics(name: &str, topics: &[String]) -> String {
    let words: HashSet<String> = topics.iter().flat_map(|t| fine(t)).map(|t| t.text).collect();
    let (stem, ext) = split_ext(name);
    let mut seen = HashSet::new();
    let repeats: Vec<(usize, usize)> = fine(stem).into_iter().filter(|t| words.contains(&t.text) && !seen.insert(t.text.clone())).map(|t| (t.start, t.end)).collect();
    let mut stem = stem.to_string();
    for &(s, e) in repeats.iter().rev() {
        match drop_span(&stem, s, e) {
            Some(rest) => stem = rest,
            None => break,
        }
    }
    format!("{stem}{ext}")
}

// Terms, excluded words and pattern matches leave the name; topics stay once and `only` terms stay as they are.
pub fn renamed(entry: &Entry, name: &str) -> String {
    let mut name = name.to_string();
    for term in entry.terms.iter().chain(&entry.exclude) {
        name = cut_all(&name, |stem| find_term(stem, term));
    }
    for pattern in &entry.patterns {
        name = cut_all(&name, |stem| pattern.find(stem).map(|m| (m.start(), m.end())));
    }
    dedupe_topics(&drop_empty_brackets(&name), &entry.topics)
}

// "chihiro (blue archive) 624658" loses its term and would keep a bare "( )".
fn drop_empty_brackets(name: &str) -> String {
    let (stem, ext) = split_ext(name);
    let mut stem = stem.to_string();
    for empty in ["( )", "()", "[ ]", "[]"] {
        while let Some(s) = stem.find(empty) {
            match drop_span(&stem, s, s + empty.len()) {
                Some(rest) => stem = rest,
                None => break,
            }
        }
    }
    format!("{stem}{ext}")
}
