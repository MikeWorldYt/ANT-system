// Turns the user's storage name into the Pascal_Snake_Case folder name used on disk.
const FORBIDDEN: &str = "\\/:*?\"<>|";

pub fn sanitize_name(raw: &str) -> String {
    let cleaned: String = raw.trim().chars().filter(|c| !FORBIDDEN.contains(*c)).collect();
    cleaned
        .split(|c: char| c.is_whitespace() || c == '_')
        .filter(|p| !p.is_empty())
        .map(capitalize)
        .collect::<Vec<_>>()
        .join("_")
}

// Matches Python's str.capitalize: first letter upper, the rest lower.
fn capitalize(part: &str) -> String {
    let mut chars = part.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars.flat_map(char::to_lowercase)).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_words_in_pascal_snake_case() {
        assert_eq!(sanitize_name("  my   storage_name "), "My_Storage_Name");
        assert_eq!(sanitize_name("HELLO__world"), "Hello_World");
    }

    #[test]
    fn drops_forbidden_characters() {
        assert_eq!(sanitize_name("a:b*c? d<e>|"), "Abc_De");
        assert_eq!(sanitize_name("\\/:*?\"<>|"), "");
    }

    #[test]
    fn handles_non_ascii_letters() {
        assert_eq!(sanitize_name("élan ÑANDÚ"), "Élan_Ñandú");
    }
}
