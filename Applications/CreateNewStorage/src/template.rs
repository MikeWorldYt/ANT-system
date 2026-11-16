// Reads the Markdown template (## categories, - topics) and numbers it into the folder tree the storage is built from.
use std::collections::HashSet;

use crate::plan::check_names;
use crate::tree_types::TreeNode;

const MAX_LEVEL: usize = 3;
const MAX_CATEGORIES: usize = 9;
const MAX_TOPICS: usize = 89;
const SUPPORT: [(&str, &str); 3] = [("REV", "REV"), ("RES", "RES"), ("TPL", "TPL")];

// Folders every root gets besides the categories.
const ROOT_FOLDERS: [(&str, &str); 2] = [("ANT.RECORDINGS", "REC-L1"), ("ANT.SCREENSHOTS", "SS-L1")];

struct Item {
    line: usize,
    level: usize,
    text: String,
}

fn fail<T>(line: usize, why: &str) -> Result<T, String> {
    Err(format!("Template line {line}: {why}"))
}

pub fn tree_from_template(file: &[u8]) -> Result<Vec<TreeNode>, String> {
    let text = std::str::from_utf8(file).map_err(|_| "The template is not UTF-8 text".to_string())?;
    let items = items_of(text)?;
    if items.is_empty() {
        return Err("The template has no `##` category".to_string());
    }
    let (mut at, mut projects) = (0, 0);
    let mut tree = nodes(&items, &mut at, 1, "", &mut projects)?;
    for category in &mut tree {
        add_category_folders(category);
        for child in &mut category.children {
            add_rev(child, 2);
        }
    }
    tree.extend(ROOT_FOLDERS.map(|(name, kind)| node(name.to_string(), kind.to_string(), Vec::new())));
    check_names(&tree)?;
    Ok(tree)
}

// Subcategories and topics get their review folder; a template `REV` item makes the same name, so it is not added twice.
fn add_rev(item: &mut TreeNode, level: usize) {
    if item.base_kind() != "NODE" {
        return;
    }
    for child in &mut item.children {
        add_rev(child, level + 1);
    }
    let name = format!("REV.{}", item.name.split_once('.').map_or("", |(_, n)| n));
    if !item.children.iter().any(|c| c.name.eq_ignore_ascii_case(&name)) {
        item.children.push(node(name, format!("REV-L{}", level + 1), Vec::new()));
    }
}

// A template `REV` at level 2 already makes `REV.<CAT>`, so existing names are kept.
fn add_category_folders(category: &mut TreeNode) {
    let cat = category.name.split_once('.').map_or("", |(_, c)| c).to_string();
    for (name, kind) in [(format!("REC.{cat}"), "REC-L2"), ("RES.Resources".to_string(), "RES-L2"), (format!("REV.{cat}"), "REV-L2"), (format!("SS.{cat}"), "SS-L2")] {
        if !category.children.iter().any(|c| c.name.eq_ignore_ascii_case(&name)) {
            category.children.push(node(name, kind.to_string(), Vec::new()));
        }
    }
}

fn items_of(text: &str) -> Result<Vec<Item>, String> {
    let mut items: Vec<Item> = Vec::new();
    for (i, raw) in text.trim_start_matches('\u{feff}').lines().enumerate() {
        let line = i + 1;
        if let Some(rest) = raw.strip_prefix("##").filter(|r| r.is_empty() || r.starts_with(char::is_whitespace)) {
            let name = checked_name(line, rest.trim())?;
            items.push(Item { line, level: 1, text: name });
            continue;
        }
        let body = raw.trim_start_matches([' ', '\t']);
        let Some(rest) = body.strip_prefix('-').filter(|r| r.is_empty() || r.starts_with(' ')) else { continue };
        let lead = &raw[..raw.len() - body.len()];
        if lead.contains('\t') {
            return fail(line, "indent with spaces, not tabs");
        }
        if items.is_empty() {
            return fail(line, "list item outside a category (add a `##` heading first)");
        }
        if lead.len() % 2 != 0 {
            return fail(line, "indentation must be a multiple of 2 spaces");
        }
        let level = lead.len() / 2 + 2;
        if level > MAX_LEVEL {
            return fail(line, "nesting deeper than 3 levels (category, subcategory, topic)");
        }
        if level > items[items.len() - 1].level + 1 {
            return fail(line, "indentation jumps more than one level");
        }
        items.push(Item { line, level, text: checked_name(line, rest.trim())? });
    }
    Ok(items)
}

fn checked_name(line: usize, text: &str) -> Result<String, String> {
    match text {
        "" => fail(line, "empty name"),
        t if t.contains(['/', '\\', '\0']) => fail(line, "a name can't contain / \\ or NUL"),
        t => Ok(t.to_string()),
    }
}

fn nodes(items: &[Item], at: &mut usize, level: usize, parent: &str, projects: &mut usize) -> Result<Vec<TreeNode>, String> {
    let (mut out, mut seen, mut count) = (Vec::new(), HashSet::new(), 0);
    while let Some(item) = items.get(*at).filter(|i| i.level == level) {
        *at += 1;
        let support = SUPPORT.iter().find(|(word, _)| item.text.eq_ignore_ascii_case(word));
        let project = item.text.get(..3).filter(|p| p.eq_ignore_ascii_case("PJ:")).map(|_| train(&item.text[3..]));
        let is_project = project.is_some();
        let (name, kind, part, leaf) = match (support, project) {
            (Some(_), _) | (_, Some(_)) if level == 1 => return fail(item.line, "a category can't be a support folder or a project; use them inside a category"),
            (Some((word, kind)), _) => {
                (format!("{word}.{parent}"), format!("{kind}-L{level}"), String::new(), true)
            }
            (_, Some(name)) => {
                if name.is_empty() {
                    return fail(item.line, "empty project name");
                }
                *projects += 1;
                (format!("PJ.{:03}_{name}", *projects), "WKB".to_string(), name, true)
            }
            _ => {
                let part = if level == 1 { words(&item.text).map(str::to_uppercase).collect::<Vec<_>>().join("-") } else { train(&item.text) };
                if part.is_empty() {
                    return fail(item.line, "empty name");
                }
                count += 1;
                if count > if level == 1 { MAX_CATEGORIES } else { MAX_TOPICS } {
                    return fail(item.line, if level == 1 { "too many categories (100 to 900 are available)" } else { "too many items here (01 to 89 are available)" });
                }
                let name = if level == 1 { format!("{}.{part}", count * 100) } else { format!("{count:02}.{part}") };
                (name, format!("NODE-L{level}"), part, false)
            }
        };
        let key = match (is_project, leaf) {
            (true, _) => format!("pj:{part}"),
            (_, true) => name.clone(),
            _ => part.clone(),
        }
        .to_lowercase();
        if !seen.insert(key) {
            return fail(item.line, &format!("duplicate name {:?} among its siblings", item.text));
        }
        let deeper = items.get(*at).filter(|next| next.level > level);
        let children = match (deeper, leaf) {
            (Some(next), true) => return fail(next.line, &format!("{:?} is a support folder or project and can't contain items", item.text)),
            (Some(_), false) => nodes(items, at, level + 1, &part, projects)?,
            (None, _) => Vec::new(),
        };
        out.push(node(name, kind, children));
    }
    Ok(out)
}

fn node(name: String, kind: String, children: Vec<TreeNode>) -> TreeNode {
    TreeNode { name, kind, fields: Vec::new(), children }
}

fn words(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| c.is_whitespace() || c == '-' || c == '_').filter(|w| !w.is_empty())
}

// Only the first letter changes, so acronyms such as "CV" survive.
fn train(text: &str) -> String {
    let capitalized = words(text).map(|w| {
        let mut chars = w.chars();
        chars.next().map(|first| first.to_uppercase().chain(chars).collect::<String>()).unwrap_or_default()
    });
    capitalized.collect::<Vec<_>>().join("-")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TEMPLATE;

    fn parse(text: &str) -> Result<Vec<TreeNode>, String> {
        tree_from_template(text.as_bytes())
    }

    fn named<'a>(nodes: &'a [TreeNode], name: &str) -> &'a TreeNode {
        nodes.iter().find(|n| n.name == name).unwrap_or_else(|| panic!("{name} not found in {:?}", nodes.iter().map(|n| &n.name).collect::<Vec<_>>()))
    }

    fn names(nodes: &[TreeNode]) -> Vec<&str> {
        nodes.iter().map(|n| n.name.as_str()).collect()
    }

    fn error(text: &str) -> String {
        parse(text).unwrap_err()
    }

    #[test]
    fn the_real_template_gives_the_numbered_tree() {
        let tree = tree_from_template(TEMPLATE).unwrap();
        assert_eq!(names(&tree), ["100.PERSONAL", "200.WORK", "300.STUDY", "400.HOBBIES", "ANT.RECORDINGS", "ANT.SCREENSHOTS"]);
        let documents = named(&named(&tree, "100.PERSONAL").children, "01.Documents");
        assert!(tree.iter().all(|n| !n.name.starts_with("900") && !n.name.starts_with("999")));
        assert_eq!(names(&documents.children), ["01.Identity", "02.Finance", "03.Health", "04.Legal", "REV.Documents"]);
        assert_eq!(documents.children[4].kind, "REV-L3");
        assert_eq!(names(&named(&documents.children, "01.Identity").children), ["REV.Identity"]);
        let projects = named(&named(&tree, "200.WORK").children, "03.Projects");
        assert_eq!(names(&projects.children), ["PJ.001_Example-Project", "REV.Projects"]);
        assert_eq!(projects.children[0].kind, "WKB");
        let research = named(&named(&tree, "300.STUDY").children, "03.Research");
        assert_eq!(names(&research.children), ["PJ.002_Example-Thesis", "REV.Research"]);
        let creative = named(&named(&tree, "400.HOBBIES").children, "04.Creative");
        assert_eq!(names(&creative.children), ["01.Drawing", "02.Crafts", "REV.Creative"]);
    }

    #[test]
    fn everything_outside_headings_and_dashes_is_ignored() {
        let tree = parse("# Title\n\n> - quoted\nplain text\n\n## a b\n- my topic\n  - x\n### not read\n1. nor this\n").unwrap();
        assert_eq!(names(&tree), ["100.A-B", "ANT.RECORDINGS", "ANT.SCREENSHOTS"]);
        assert_eq!(names(&tree[0].children), ["01.My-Topic", "REC.A-B", "RES.Resources", "REV.A-B", "SS.A-B"]);
        assert_eq!(names(&tree[0].children[0].children), ["01.X", "REV.My-Topic"]);
    }

    #[test]
    fn names_follow_block_and_chill_naming() {
        let tree = parse("\u{feff}## first\r\n## second\r\n- online courses\r\n- CV\r\n  - snake_case\r\n").unwrap();
        assert_eq!(names(&tree), ["100.FIRST", "200.SECOND", "ANT.RECORDINGS", "ANT.SCREENSHOTS"]);
        assert_eq!(names(&tree[1].children)[..2], ["01.Online-Courses", "02.CV"]);
        assert_eq!(names(&tree[1].children[1].children), ["01.Snake-Case", "REV.CV"]);
    }

    #[test]
    fn project_numbers_are_global_and_three_digits() {
        let tree = parse("## a\n- x\n  - PJ: first one\n  - pj:Second\n## b\n- y\n  - PJ: third\n").unwrap();
        assert_eq!(names(&tree[0].children[0].children), ["PJ.001_First-One", "PJ.002_Second", "REV.X"]);
        assert_eq!(names(&tree[1].children[0].children), ["PJ.003_Third", "REV.Y"]);
    }

    #[test]
    fn reserved_items_are_support_folders_that_skip_the_counter() {
        let tree = parse("## work\n- rev\n- Dev\n  - RES\n  - tpl\n  - Ideas\n").unwrap();
        let level2 = &tree[0].children;
        assert_eq!(names(level2), ["REV.WORK", "01.Dev", "REC.WORK", "RES.Resources", "SS.WORK"]);
        assert_eq!(names(&level2[1].children), ["RES.Dev", "TPL.Dev","01.Ideas", "REV.Dev"]);
        assert_eq!(level2[0].kind, "REV-L2");
        assert_eq!(level2[1].children[1].kind, "TPL-L3");
    }

    #[test]
    fn every_category_gets_four_folders_and_the_root_two() {
        let tree = parse("## personal\n- docs\n## my work\n").unwrap();
        for (cat, tag) in [(0, "PERSONAL"), (1, "MY-WORK")] {
            let found = names(&tree[cat].children);
            for want in [format!("REC.{tag}"), "RES.Resources".to_string(), format!("REV.{tag}"), format!("SS.{tag}")] {
                assert_eq!(found.iter().filter(|n| **n == want).count(), 1, "{want} in {found:?}");
            }
        }
        assert_eq!(named(&tree[0].children, "SS.PERSONAL").kind, "SS-L2");
        assert!(tree[2..].iter().all(|n| n.children.is_empty()) && names(&tree[2..]) == ["ANT.RECORDINGS", "ANT.SCREENSHOTS"]);
        assert!(!names(&tree).iter().any(|n| n.starts_with("900") || n.starts_with("999")));
    }

    #[test]
    fn review_folders_are_automatic_for_subcategories_and_topics_only() {
        let tree = parse("## a\n- x\n  - y\n  - PJ: p\n  - REV\n- PJ: q\n").unwrap();
        let x = named(&tree[0].children, "01.X");
        assert_eq!(names(&x.children), ["01.Y", "PJ.001_P", "REV.X"]);
        assert_eq!(x.children[2].kind, "REV-L3");
        assert_eq!(names(&x.children[0].children), ["REV.Y"]);
        assert!(x.children[1].children.is_empty());
        assert!(named(&tree[0].children, "PJ.002_Q").children.is_empty());
        assert!(named(&tree[0].children, "SS.A").children.is_empty());
    }

    #[test]
    fn reserved_items_and_projects_are_refused_as_categories() {
        for text in ["## REV\n", "## res\n", "## TPL\n","## PJ: a\n"] {
            assert!(error(text).starts_with("Template line 1:"), "{text}");
        }
    }

    #[test]
    fn structure_errors_name_the_line() {
        assert!(error("").contains("no `##` category"));
        assert!(error("> only text\n").contains("no `##` category"));
        assert!(error("- a\n## b\n").starts_with("Template line 1:") && error("- a\n").contains("outside a category"));
        assert!(error("## a\n- b\n  - c\n    - d\n").starts_with("Template line 4:") && error("## a\n- b\n  - c\n    - d\n").contains("deeper than 3"));
        assert!(error("## a\n - b\n").starts_with("Template line 2:") && error("## a\n - b\n").contains("multiple of 2"));
        assert!(error("## a\n  - b\n").contains("jumps"));
        assert!(error("## a\n- b\n\t- c\n").contains("tabs"));
        assert!(error("## \n").contains("empty name"));
        assert!(error("## a\n-\n").starts_with("Template line 2:"));
        assert!(error("## a\n- ---\n").contains("empty name"));
        assert!(error("## a\n- x\n  - PJ:  \n").contains("empty project name"));
    }

    #[test]
    fn bad_characters_duplicates_and_children_of_leaves_are_refused() {
        assert!(error("## a/b\n").contains("can't contain"));
        assert!(error("## a\n- b\\c\n").starts_with("Template line 2:"));
        assert!(error("## a\n- Docs\n- docs\n").contains("duplicate"));
        assert!(error("## a\n## A\n").contains("duplicate"));
        assert!(error("## a\n- x\n  - REV\n  - rev\n").contains("duplicate"));
        assert!(error("## a\n- REV\n  - x\n").starts_with("Template line 3:"));
        assert!(error("## a\n- x\n  - PJ: p\n    - y\n").contains("deeper than 3"));
        assert!(error("## a\n- PJ: p\n  - y\n").contains("can't contain"));
    }

    #[test]
    fn counters_stop_at_their_limits() {
        let ten: String = (1..=10).map(|n| format!("## c{n}\n")).collect();
        assert!(error(&ten).starts_with("Template line 10:") && error(&ten).contains("100 to 900"));
        let nine = parse(&ten[..ten.len() - 7]).unwrap();
        assert_eq!(nine[8].name, "900.C9");
        let ninety: String = std::iter::once("## a\n".to_string()).chain((1..=90).map(|n| format!("- t{n}\n"))).collect();
        assert!(error(&ninety).starts_with("Template line 91:"));
    }

    #[test]
    fn a_file_that_is_not_utf8_is_refused() {
        assert!(tree_from_template(&[0xff, 0xfe, b'#']).unwrap_err().contains("UTF-8"));
    }
}
