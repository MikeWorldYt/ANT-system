// Search query syntax: `in:200.WORK client:Mike status: in-progress plain words`.
use crate::storage::info::OWNER_KEYS;

// The project owner can be searched by any of these names.
pub fn canonical_key(key: &str) -> String {
    let key = key.to_lowercase();
    if OWNER_KEYS.contains(&key.as_str()) { OWNER_KEYS[0].to_string() } else { key }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Query {
    pub scope: Option<String>,
    pub filters: Vec<(String, String)>,
    pub text: Vec<String>,
}

pub fn parse_query(input: &str) -> Query {
    let mut query = Query::default();
    let mut tokens = split_tokens(input).into_iter();
    while let Some(token) = tokens.next() {
        match token.split_once(':') {
            Some((key, rest)) if !key.is_empty() && !key.contains('"') => {
                // "status: in-progress" puts the value in the next token.
                let value = if rest.is_empty() { tokens.next().unwrap_or_default() } else { rest.to_string() };
                let value = unquote(&value);
                if value.is_empty() {
                    continue;
                }
                if key.eq_ignore_ascii_case("in") {
                    query.scope = Some(value);
                } else {
                    query.filters.push((canonical_key(key), value));
                }
            }
            _ => query.text.push(unquote(&token)),
        }
    }
    query
}

fn split_tokens(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    for c in input.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                current.push(c);
            }
            c if c.is_whitespace() && !quoted => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            c => current.push(c),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn unquote(s: &str) -> String {
    s.trim_matches('"').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair(k: &str, v: &str) -> (String, String) {
        (k.to_string(), v.to_string())
    }

    #[test]
    fn scope_and_aliased_filter() {
        let q = parse_query("in:200.WORK client:Mike");
        assert_eq!(q.scope.as_deref(), Some("200.WORK"));
        assert_eq!(q.filters, [pair("owner", "Mike")]);
        assert!(q.text.is_empty());
    }

    #[test]
    fn a_space_after_the_colon_still_binds_the_value() {
        assert_eq!(parse_query("status: in-progress").filters, [pair("status", "in-progress")]);
    }

    #[test]
    fn quoted_values_and_plain_words() {
        let q = parse_query(r#"client:"Mike W" fabi "two words""#);
        assert_eq!(q.filters, [pair("owner", "Mike W")]);
        assert_eq!(q.text, ["fabi", "two words"]);
    }

    #[test]
    fn keys_are_case_insensitive_and_empty_filters_are_dropped() {
        let q = parse_query("Status:Hold in:");
        assert_eq!(q.filters, [pair("status", "Hold")]);
        assert_eq!(q.scope, None);
    }
}
