// Turns the template database's nodes and fields into the tree the storage is built from.
use std::collections::HashMap;

use crate::info::FIELDS;
use crate::plan::check_names;
use crate::sqlite::{self, Database, Row, Table};
use crate::tree_types::TreeNode;

const MAX_DEPTH: usize = 64;

type Fields = HashMap<String, Vec<(String, String)>>;

struct Node {
    id: i64,
    parent: i64,
    name: String,
    kind: String,
}

struct Template {
    nodes: Vec<Node>,
    by_parent: HashMap<i64, Vec<usize>>,
    fields: Fields,
}

pub fn tree_from_template(file: &[u8]) -> Result<Vec<TreeNode>, String> {
    let db = Database::open(file)?;
    let mut nodes = nodes_of(&db.table("nodes")?)?;
    nodes.sort_by_key(|n| n.id);
    let fields = fields_of(&db.table("node_fields")?)?;
    let mut by_parent: HashMap<i64, Vec<usize>> = HashMap::new();
    for (i, node) in nodes.iter().enumerate() {
        by_parent.entry(node.parent).or_default().push(i);
    }
    let count = nodes.len();
    let template = Template { nodes, by_parent, fields };
    let mut reached = 0;
    let tree = template.children(0, "", &mut reached, 0)?;
    if reached != count {
        return Err("ant.db template has entries outside its tree".to_string());
    }
    check_names(&tree)?;
    Ok(tree)
}

fn nodes_of(table: &Table) -> Result<Vec<Node>, String> {
    let (parent_at, name_at, kind_at) = (column(table, "parent_id")?, column(table, "name")?, column(table, "type")?);
    table
        .rows
        .iter()
        .map(|row| -> Result<Node, String> {
            let parent =row.values.get(parent_at).and_then(sqlite::Value::as_int).ok_or_else(|| "ant.db template has a node without a parent".to_string())?;
            Ok(Node { id: row.rowid, parent, name: text(row, name_at).to_string(), kind: text(row, kind_at).to_string() })
        })
        .collect()
}

fn fields_of(table: &Table) -> Result<Fields, String> {
    let (path_at, key_at, value_at) = (column(table, "path")?, column(table, "key")?, column(table, "value")?);
    let mut fields: Fields = HashMap::new();
    for row in &table.rows {
        let key = text(row, key_at);
        if FIELDS.contains(&key) {
            fields.entry(text(row, path_at).to_string()).or_default().push((key.to_string(), text(row, value_at).to_string()));
        }
    }
    Ok(fields)
}

fn column(table: &Table, name: &str) -> Result<usize, String> {
    table.columns.iter().position(|c| c == name).ok_or_else(|| format!("ant.db template has no {name} column"))
}

fn text(row: &Row, at: usize) -> &str {
    row.values.get(at).and_then(sqlite::Value::as_text).unwrap_or_default()
}

impl Template {
    fn children(&self, parent: i64, prefix: &str, reached: &mut usize, depth: usize) -> Result<Vec<TreeNode>, String> {
        let Some(kids) = self.by_parent.get(&parent) else { return Ok(Vec::new()) };
        if depth > MAX_DEPTH {
            return Err("ant.db template is nested too deeply".to_string());
        }
        let mut out = Vec::with_capacity(kids.len());
        for &index in kids {
            let node = &self.nodes[index];
            *reached += 1;
            let path = if prefix.is_empty() { node.name.clone() } else { format!("{prefix}/{}", node.name) };
            let children = self.children(node.id, &path, reached, depth + 1)?;
            let fields = self.fields.get(&path).cloned().unwrap_or_default();
            out.push(TreeNode { name: node.name.clone(), kind: node.kind.clone(), fields, children });
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TEMPLATE;

    fn named<'a>(nodes: &'a [TreeNode], name: &str) -> &'a TreeNode {
        nodes.iter().find(|n| n.name == name).unwrap()
    }

    #[test]
    fn the_starter_template_gives_its_tree_and_info_fields() {
        let tree = tree_from_template(TEMPLATE).unwrap();
        let work = named(&tree, "200.WORK");
        let industry = named(&work.children, "02.Industry");
        let pj = named(&industry.children, "PJ.001_project-name-example");
        assert_eq!(pj.kind, "WKB");
        assert!(pj.fields.contains(&("created".to_string(), "03-01-03".to_string())));
    }

    #[test]
    fn a_file_that_is_not_a_database_is_refused() {
        assert!(tree_from_template(b"name: not a database").is_err());
    }
}
