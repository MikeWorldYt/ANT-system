// Strips noise codes (hashes, site ids, long non-date digit runs) from file names, keeping 4 chars per code.
// Port of the Reformatter's truncate_long_name.py; the regexes are hand-written because the crate has no regex dependency.
mod dated;
mod md5;

use std::collections::HashSet;

const KEEP: usize = 4;
const GENERIC: [&str; 5] = ["image", "images", "download", "unnamed", "untitled"];
const NOISE_KEYWORDS: [&str; 14] = ["ezgif", "facebook", "fb_img", "pixiv", "pixai", "yande.re", "yander.re", "ssstiktok", "tiktok", "pexels", "whatsapp", "image", "video", "sample"];
// First code point of every Unicode decimal-digit block (Python's `\d`), each block spanning 10 digits.
const DIGIT_BLOCKS: [u32; 68] = [
    0x30, 0x660, 0x6F0, 0x7C0, 0x966, 0x9E6, 0xA66, 0xAE6, 0xB66, 0xBE6, 0xC66, 0xCE6, 0xD66, 0xDE6, 0xE50, 0xED0, 0xF20, 0x1040, 0x1090, 0x17E0, 0x1810, 0x1946, 0x19D0, 0x1A80, 0x1A90, 0x1B50, 0x1BB0, 0x1C40,
    0x1C50, 0xA620, 0xA8D0, 0xA900, 0xA9D0, 0xA9F0, 0xAA50, 0xABF0, 0xFF10, 0x104A0, 0x10D30, 0x11066, 0x110F0, 0x11136, 0x111D0, 0x112F0, 0x11450, 0x114D0, 0x11650, 0x116C0, 0x11730, 0x118E0, 0x11950,
    0x11C50, 0x11D50, 0x11DA0, 0x11F50, 0x16A60, 0x16AC0, 0x16B50, 0x1D7CE, 0x1D7D8, 0x1D7E2, 0x1D7EC, 0x1D7F6, 0x1E140, 0x1E2F0, 0x1E4F0, 0x1E950, 0x1FBF0,
];

type Replaced = Option<(usize, Vec<char>)>;

fn digit(c: char) -> Option<u32> {
    if c.is_ascii() {
        return c.to_digit(10);
    }
    DIGIT_BLOCKS.iter().find(|&&s| (s..s + 10).contains(&(c as u32))).map(|s| c as u32 - s)
}

fn is_digit(c: char) -> bool {
    digit(c).is_some()
}

fn is_sep(c: char) -> bool {
    matches!(c, '_' | '-' | ' ')
}

fn num(s: &[char]) -> u32 {
    s.iter().filter_map(|&c| digit(c)).fold(0, |n, d| n * 10 + d)
}

// Python's re.I also folds these four non-ASCII letters onto ASCII ones.
fn folded(c: char) -> char {
    match c {
        'ſ' => 's',
        '\u{212A}' => 'k',
        'ı' | 'İ' => 'i',
        _ => c.to_ascii_lowercase(),
    }
}

fn lit(s: &[char], at: usize, text: &str) -> Option<usize> {
    let mut i = at;
    for k in text.chars() {
        (s.get(i) == Some(&k)).then_some(())?;
        i += 1;
    }
    Some(i)
}

fn lit_ci(s: &[char], at: usize, text: &str) -> Option<usize> {
    let mut i = at;
    for k in text.chars() {
        (s.get(i).map(|&c| folded(c)) == Some(k)).then_some(())?;
        i += 1;
    }
    Some(i)
}

fn digit_run_end(s: &[char], from: usize) -> usize {
    from + s[from.min(s.len())..].iter().take_while(|&&c| is_digit(c)).count()
}

fn sub(s: &[char], matcher: impl Fn(&[char], usize) -> Replaced) -> Vec<char> {
    let (mut out, mut i) = (Vec::with_capacity(s.len()), 0);
    while i < s.len() {
        match matcher(s, i) {
            Some((end, rep)) => {
                out.extend(rep);
                i = end;
            }
            None => {
                out.push(s[i]);
                i += 1;
            }
        }
    }
    out
}

fn is_counter(t: &[char]) -> bool {
    matches!(t, [' ', '(', mid @ .., ')'] if !mid.is_empty() && mid.iter().all(|&c| is_digit(c)))
}

fn counter_start(s: &[char]) -> Option<usize> {
    (0..s.len()).find(|&i| is_counter(&s[i..]))
}

fn is_generic(stem: &[char]) -> bool {
    GENERIC.iter().any(|kw| lit_ci(stem, 0, kw).is_some_and(|e| e == stem.len() || is_counter(&stem[e..])))
}

fn noise(s: &[char], i: usize) -> Replaced {
    for lead in [0, 1] {
        if (lead == 0 && i != 0) || (lead == 1 && !s.get(i).is_some_and(|&c| is_sep(c))) {
            continue;
        }
        for kw in NOISE_KEYWORDS {
            if let Some(e) = lit_ci(s, i + lead, kw).filter(|&e| e == s.len() || is_sep(s[e])) {
                return Some((e, vec![]));
            }
        }
    }
    None
}

fn paren_counter(s: &[char], i: usize) -> Replaced {
    let end = digit_run_end(s, lit(s, i, "(")?);
    (end > i + 1 && s.get(end) == Some(&')')).then(|| (end + 1, vec![]))
}

fn has_name(s: &[char]) -> bool {
    sub(s, paren_counter).iter().any(char::is_ascii_alphanumeric)
}

fn trailing_n(s: &[char], i: usize) -> Replaced {
    let after = lit(s, i, "_n").filter(|_| i > 0 && s[i - 1].is_ascii_alphanumeric())?;
    let end = after + s[after..].iter().take_while(|&&c| is_sep(c)).count();
    (end == s.len()).then(|| (end, vec![]))
}

fn reddit(s: &[char], i: usize) -> Replaced {
    let from = lit(s, i, "-v0-")?;
    let end = from + s[from..].iter().take_while(|c| c.is_ascii_digit() || c.is_ascii_lowercase()).count();
    (end > from && end == s.len()).then(|| (end, vec![]))
}

fn trailing_numbers(s: &[char], i: usize) -> Replaced {
    let first = digit_run_end(s, lit(s, i, ".")?);
    let second = digit_run_end(s, lit(s, first, ".")?);
    // "Ganyu.600.3545662": the size code goes and the id keeps its first chars, so copies stay apart.
    ((3..=4).contains(&(first - i - 1)) && second - first - 1 >= 4 && second == s.len()).then(|| (second, std::iter::once('_').chain(s[first + 1..first + 1 + KEEP].iter().copied()).collect()))
}

fn uuid(s: &[char], i: usize) -> Replaced {
    let mut j = i;
    for (k, n) in [8, 4, 4, 4, 12].into_iter().enumerate() {
        if k > 0 {
            j = lit(s, j, "-")?;
        }
        for _ in 0..n {
            s.get(j).filter(|c| c.is_ascii_hexdigit())?;
            j += 1;
        }
    }
    Some((j, s[i..i + KEEP].to_vec()))
}

fn is_code(t: &[char]) -> bool {
    let is_date = matches!(t.len(), 15 | 16)
        && t[..8].iter().all(char::is_ascii_digit)
        && t[8] == 'T'
        && t[9..15].iter().all(char::is_ascii_digit)
        && (t.len() == 15 || t[15] == 'Z');
    let digits = t.iter().filter(|c| c.is_ascii_digit()).count();
    let letters = t.len() - digits;
    if is_date || digits == 0 || letters == 0 {
        return false;
    }
    if t.iter().all(char::is_ascii_hexdigit) {
        return true;
    }
    let switches = t.windows(2).filter(|w| w[0].is_ascii_digit() != w[1].is_ascii_digit()).count();
    digits >= 3 && letters >= 3 && digits * 4 >= t.len() && switches >= 6
}

fn code(s: &[char], i: usize) -> Replaced {
    let underscores = s[i..].iter().take_while(|&&c| c == '_').count();
    // "sample" goes with whatever symbol follows it: "sample-", "sample_", "sample*".
    let prefixed = lit_ci(s, i + underscores, "sample").map(|e| e + usize::from(s.get(e).is_some_and(|c| !c.is_alphanumeric())));
    let bare = (i == 0 || !s[i - 1].is_ascii_alphanumeric()).then_some(i);
    for from in [prefixed, bare].into_iter().flatten() {
        let end = from + s[from..].iter().take_while(|c| c.is_ascii_alphanumeric()).count();
        if end - from < 16 {
            continue;
        }
        let token = &s[from..end];
        if !is_code(token) {
            return Some((end, s[i..end].to_vec()));
        }
        // A removed "__sample-" prefix leaves the title glued to the code
        let glued = from != i && i > 0 && s[i - 1].is_alphanumeric();
        let mut rep = if glued { vec!['_'] } else { vec![] };
        rep.extend(&token[..KEEP]);
        return Some((end, rep));
    }
    None
}

fn valid_ymd(year: u32, month: u32, day: u32) -> bool {
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = match month {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    (1900..=2099).contains(&year) && (1..=12).contains(&month) && (1..=days).contains(&day)
}

fn is_date_run(r: &[char]) -> bool {
    if r.len() == 8 {
        return valid_ymd(num(&r[..4]), num(&r[4..6]), num(&r[6..])) || valid_ymd(num(&r[4..]), num(&r[2..4]), num(&r[..2]));
    }
    if ![12, 14, 17].contains(&r.len()) || !valid_ymd(num(&r[..4]), num(&r[4..6]), num(&r[6..8])) {
        return false;
    }
    num(&r[8..10]) < 24 && num(&r[10..12]) < 60 && (r.len() == 12 || num(&r[12..14]) < 60)
}

fn is_time_after_date(s: &[char], start: usize, end: usize) -> bool {
    let (run, prev) = (&s[start..end], &s[..start]);
    let n = prev.len();
    run.len() == 9
        && num(&run[..2]) < 24
        && num(&run[2..4]) < 60
        && num(&run[4..6]) < 60
        && n >= 9
        && is_sep(prev[n - 1])
        && prev[n - 9..n - 1].iter().all(|&c| is_digit(c))
        && (n == 9 || !is_digit(prev[n - 10]))
        && is_date_run(&prev[n - 9..n - 1])
}

fn long_runs(s: &[char]) -> Vec<(usize, usize)> {
    let (mut runs, mut i) = (vec![], 0);
    while i < s.len() {
        if is_digit(s[i]) {
            let end = digit_run_end(s, i);
            if end - i >= 8 {
                runs.push((i, end));
            }
            i = end;
        } else {
            i += 1;
        }
    }
    runs
}

fn has_iso_date(s: &[char]) -> bool {
    let digits = |from: usize, n: usize| (from..from + n).all(|k| s.get(k).is_some_and(|&c| is_digit(c)));
    (0..s.len()).any(|i| {
        (i == 0 || !is_digit(s[i - 1]))
            && digits(i, 4)
            && s.get(i + 4) == Some(&'-')
            && digits(i + 5, 2)
            && s.get(i + 7) == Some(&'-')
            && digits(i + 8, 2)
            && !s.get(i + 10).is_some_and(|&c| is_digit(c))
    })
}

fn is_numeric_name(b: &[char]) -> bool {
    b.first().is_some_and(|&c| is_digit(c)) && b.last().is_some_and(|&c| is_digit(c)) && b.iter().all(|&c| is_digit(c) || is_sep(c))
}

fn cut_long_runs(s: &[char]) -> Vec<char> {
    let runs = long_runs(s);
    let kept = |&(a, b): &(usize, usize)| is_date_run(&s[a..b]) || is_time_after_date(s, a, b);
    let ids = runs.iter().filter(|r| !kept(*r)).count();
    if ids == 0 {
        return s.to_vec();
    }
    let has_date = ids < runs.len() || has_iso_date(s);
    let (mut cut, mut at) = (vec![], 0);
    for run in &runs {
        let (a, b) = *run;
        cut.extend(&s[at..a]);
        cut.extend(&s[a..if kept(run) { b } else { a + KEEP }]);
        at = b;
    }
    cut.extend(&s[at..]);
    let counter = counter_start(&cut);
    let body = &cut[..counter.unwrap_or(cut.len())];
    let body = &body[body.iter().take_while(|&&c| is_sep(c)).count()..];
    if has_date || !is_numeric_name(body) {
        return cut;
    }
    let first = body.iter().position(|&c| is_sep(c)).unwrap_or(body.len());
    let mut out = body[..first.min(KEEP)].to_vec();
    out.extend(counter.map_or(&[][..], |c| &cut[c..]));
    out
}

fn collapse_separators(s: &[char]) -> Vec<char> {
    let mut out: Vec<char> = Vec::with_capacity(s.len());
    for &c in s {
        if !(is_sep(c) && out.last() == Some(&c)) {
            out.push(c);
        }
    }
    out
}

const EMBEDDED_EXTS: [&str; 22] = ["jpg", "jpeg", "png", "gif", "webp", "jfif", "bmp", "tif", "tiff", "heic", "avif", "svg", "mp4", "mkv", "mov", "webm", "avi", "mp3", "wav", "flac", "pdf", "zip"];
const SYMBOLS: [char; 6] = ['_', '-', ' ', '.', '+', '~'];

// "thumb.jpeg.13bc" and "6298292_jpg" carry an old extension in the stem; only the real one stays.
fn drop_embedded_exts(s: &[char]) -> Vec<char> {
    let text: String = s.iter().collect();
    let mut out = String::new();
    let mut word = String::new();
    let flush = |word: &mut String, out: &mut String| {
        if EMBEDDED_EXTS.contains(&word.to_lowercase().as_str()) && !out.is_empty() {
            while out.ends_with(SYMBOLS) {
                out.pop();
            }
        } else {
            out.push_str(word);
        }
        word.clear();
    };
    for c in text.chars() {
        if c.is_alphanumeric() {
            word.push(c);
        } else {
            flush(&mut word, &mut out);
            out.push(c);
        }
    }
    flush(&mut word, &mut out);
    out.chars().collect()
}

// A run of mixed symbols such as "_~_" or "+_-" keeps one, preferring one that is not a space; a leading dot stays for hidden files.
fn collapse_symbol_runs(s: &[char]) -> Vec<char> {
    let mut out: Vec<char> = Vec::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        let run = s[i..].iter().take_while(|c| SYMBOLS.contains(c)).count();
        if run < 2 || i == 0 {
            out.extend(&s[i..i + run.max(1)]);
            i += run.max(1);
            continue;
        }
        out.push(s[i..i + run].iter().copied().find(|&c| c != ' ').unwrap_or(' '));
        i += run;
    }
    out
}

fn short_md5(text: &str) -> String {
    md5::hex(text.as_bytes())[..KEEP].to_string()
}

fn clean_stem(stem: &[char], original: &str) -> String {
    if is_generic(stem) {
        return short_md5(original);
    }
    let without_noise = sub(stem, noise);
    let mut s = if has_name(&without_noise) { without_noise } else { stem.to_vec() };
    s = sub(&s, trailing_n);
    s = sub(&s, reddit);
    s = sub(&s, trailing_numbers);
    s = sub(&s, uuid);
    s = sub(&s, code);
    s = collapse_symbol_runs(&collapse_separators(&cut_long_runs(&drop_embedded_exts(&s))));
    // Leading dots are kept: they mark hidden files such as ".bashrc"
    let joined: String = s.iter().collect();
    let trimmed = joined.trim_start_matches(is_sep).trim_end_matches(|c| is_sep(c) || c == '.');
    // A lone " (n)" copy counter is not a name
    if has_name(&trimmed.chars().collect::<Vec<_>>()) {
        trimmed.to_string()
    } else {
        short_md5(original)
    }
}

// pathlib splits the suffix at the last dot unless it opens or closes the name.
fn split_ext(name: &str) -> (&str, &str) {
    match name.rfind('.') {
        Some(i) if i > 0 && i < name.len() - 1 => name.split_at(i),
        _ => (name, ""),
    }
}

// The add-date automation: a name without a date gets its file's creation time in front; one with a date gets that date moved to the front.
pub fn dated_name(name: &str, created: Option<i64>) -> String {
    let (stem, suffix) = split_ext(name);
    if dated::has_date(stem) {
        return dated::split_date(stem).map_or_else(|| name.to_string(), |(date, rest)| if rest.is_empty() { format!("({date}){suffix}") } else { format!("({date})_{rest}{suffix}") });
    }
    created.map_or_else(|| name.to_string(), |secs| format!("({})_{stem}{suffix}", dated::stamp(secs)))
}

// A name that is only a date keeps it, since nothing else would be left.
pub fn undated_name(name: &str) -> String {
    let (stem, suffix) = split_ext(name);
    let rest = dated::without_date(stem);
    if rest.trim_matches(['_', '-', ' ', '.']).is_empty() { name.to_string() } else { format!("{rest}{suffix}") }
}

// The remove-symbols automation: opening and closing marks go, as owner tags "[Mike]" or a date "(26-05-04)" the user does not want.
pub fn unbracketed_name(name: &str) -> String {
    let (stem, suffix) = split_ext(name);
    let bare: Vec<char> = stem.chars().filter(|c| !matches!(c, '(' | ')' | '[' | ']' | '{' | '}')).collect();
    let joined: String = collapse_symbol_runs(&bare).into_iter().collect();
    let trimmed = joined.trim_matches(|c| is_sep(c) || c == '.');
    if trimmed.is_empty() { name.to_string() } else { format!("{trimmed}{suffix}") }
}

pub fn new_name(name: &str) -> String {
    let (stem, suffix) = split_ext(name);
    let Some((date, rest)) = dated::split_date(stem) else {
        return clean_stem(&stem.chars().collect::<Vec<_>>(), name) + suffix;
    };
    // With a date in front, a generic or empty rest adds nothing, so no hash stands in for it.
    let chars: Vec<char> = rest.chars().collect();
    let cleaned = if chars.is_empty() || is_generic(&chars) { String::new() } else { clean_stem(&chars, name) };
    let extra = if cleaned.is_empty() || cleaned == short_md5(name) { String::new() } else { format!("_{cleaned}") };
    format!("({date}){extra}{suffix}")
}

pub fn unique_name(name: &str, taken: &HashSet<String>) -> String {
    if !taken.contains(&name.to_lowercase()) {
        return name.to_string();
    }
    let (stem, suffix) = split_ext(name);
    let stem: Vec<char> = stem.chars().collect();
    let counter = counter_start(&stem);
    let mut number = counter.map_or(2, |c| stem[c + 2..stem.len() - 1].iter().filter_map(|&d| digit(d)).fold(0u128, |n, d| n.saturating_mul(10).saturating_add(d as u128)).saturating_add(1));
    let base: String = stem[..counter.unwrap_or(stem.len())].iter().collect();
    while taken.contains(&format!("{base} ({number}){suffix}").to_lowercase()) {
        number = number.saturating_add(1);
    }
    format!("{base} ({number}){suffix}")
}
