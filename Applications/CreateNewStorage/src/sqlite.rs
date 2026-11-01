// Reads table rows out of an SQLite file with std only, enough for the storage template's own tables.
const HEADER: &[u8] = b"SQLite format 3\0";
const MAX_DEPTH: usize = 64;
const RESERVED_WORDS: [&str; 5] = ["UNIQUE", "PRIMARY", "CHECK", "FOREIGN", "CONSTRAINT"];

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Int(i64),
    Text(String),
    Other,
}

impl Value {
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Value::Text(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(n) => Some(*n),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct Row {
    pub rowid: i64,
    pub values: Vec<Value>,
}

#[derive(Debug)]
pub struct Table {
    pub columns: Vec<String>,
    pub rows: Vec<Row>,
}

pub struct Database<'a> {
    file: &'a [u8],
    page_size: usize,
    usable: usize,
}

impl<'a> Database<'a> {
    pub fn open(file: &'a [u8]) -> Result<Self, String> {
        if file.len() < 100 || !file.starts_with(HEADER) {
            return Err("ant.db is not an SQLite database".to_string());
        }
        let raw = usize::from(u16::from_be_bytes([file[16], file[17]]));
        let page_size = if raw == 1 { 65536 } else { raw };
        if !page_size.is_power_of_two() || !(512..=65536).contains(&page_size) {
            return Err(format!("ant.db has an invalid page size {page_size}"));
        }
        Ok(Self { file, page_size, usable: page_size - usize::from(file[20]) })
    }

    pub fn table(&self, name: &str) -> Result<Table, String> {
        let mut master = Vec::new();
        self.collect(1, false, &mut master, 0)?;
        let entry = master
            .iter()
            .find(|r| r.values.first().and_then(Value::as_text) == Some("table") && r.values.get(1).and_then(Value::as_text) == Some(name))
            .ok_or_else(|| format!("ant.db has no {name} table"))?;
        let root = entry.values.get(3).and_then(Value::as_int).ok_or("ant.db schema is not readable")?;
        let sql = entry.values.get(4).and_then(Value::as_text).unwrap_or_default().to_string();
        let without_rowid = sql.trim_end().to_ascii_uppercase().ends_with("WITHOUT ROWID");
        let mut rows = Vec::new();
        self.collect(root as u32, without_rowid, &mut rows, 0)?;
        Ok(Table { columns: columns(&sql), rows })
    }

    fn page(&self, no: u32) -> Result<&'a [u8], String> {
        let index = (no as usize).checked_sub(1).ok_or("ant.db points at page 0")?;
        let start = index * self.page_size;
        self.file.get(start..start + self.page_size).ok_or_else(|| format!("ant.db page {no} is past the end of the file"))
    }

    // Table b-trees hold rows in leaves; WITHOUT ROWID tables use index b-trees, whose interior cells hold rows too.
    fn collect(&self, page_no: u32, index: bool, out: &mut Vec<Row>, depth: usize) -> Result<(), String> {
        if depth > MAX_DEPTH {
            return Err("ant.db tree is nested too deeply".to_string());
        }
        let page = self.page(page_no)?;
        let base = if page_no == 1 { 100 } else { 0 };
        let kind = byte_at(page, base)?;
        let (leaf, interior) = match (index, kind) {
            (false, 0x0d) | (true, 0x0a) => (true, false),
            (false, 0x05) | (true, 0x02) => (false, true),
            _ => return Err(format!("ant.db page {page_no} has an unexpected type {kind:#x}")),
        };
        let count = usize::from(be_u16(page, base + 3)?);
        let cells_at = base + if leaf { 8 } else { 12 };
        for i in 0..count {
            let at = usize::from(be_u16(page, cells_at + 2 * i)?);
            let cell = page.get(at..).ok_or("ant.db cell points outside its page")?;
            let (child, row) = match (index, leaf) {
                (false, true) => {
                    let (size, n) = varint(cell, 0)?;
                    let (rowid, m) = varint(cell, n)?;
                    (None, Some((rowid as i64, payload(cell, n + m, size, self.table_limit())?)))
                }
                (false, false) => (Some(be_u32(cell, 0)?), None),
                (true, true) => {
                    let (size, n) = varint(cell, 0)?;
                    (None, Some((0, payload(cell, n, size, self.index_limit())?)))
                }
                (true, false) => {
                    let (size, n) = varint(cell, 4)?;
                    (Some(be_u32(cell, 0)?), Some((0, payload(cell, 4 + n, size, self.index_limit())?)))
                }
            };
            if let Some(child) = child {
                self.collect(child, index, out, depth + 1)?;
            }
            if let Some((rowid, bytes)) = row {
                out.push(Row { rowid, values: record(bytes)? });
            }
        }
        if interior {
            self.collect(be_u32(page, base + 8)?, index, out, depth + 1)?;
        }
        Ok(())
    }

    // Rows that fit on their own page; a longer one would need overflow pages, which the template never uses.
    fn table_limit(&self) -> usize {
        self.usable - 35
    }

    fn index_limit(&self) -> usize {
        (self.usable - 12) * 64 / 255 - 23
    }
}

fn payload(cell: &[u8], at: usize, size: u64, limit: usize) -> Result<&[u8], String> {
    let size = usize::try_from(size).map_err(|_| "ant.db has a row that is too large".to_string())?;
    if size > limit {
        return Err("ant.db has a row on overflow pages, which the template reader does not follow".to_string());
    }
    cell.get(at..at + size).ok_or_else(|| "ant.db row is cut short".to_string())
}

fn record(bytes: &[u8]) -> Result<Vec<Value>, String> {
    let (header_len, mut at) = varint(bytes, 0)?;
    let header_len = usize::try_from(header_len).map_err(|_| "ant.db record header is too large".to_string())?;
    let mut kinds = Vec::new();
    while at < header_len {
        let (kind, n) = varint(bytes, at)?;
        kinds.push(kind);
        at += n;
    }
    let mut body = header_len;
    let mut values = Vec::with_capacity(kinds.len());
    for kind in kinds {
        let (value, n) = column_value(bytes, body, kind)?;
        values.push(value);
        body += n;
    }
    Ok(values)
}

fn column_value(bytes: &[u8], at: usize, kind: u64) -> Result<(Value, usize), String> {
    match kind {
        0 => Ok((Value::Null, 0)),
        1..=6 => {
            let width = [1, 2, 3, 4, 6, 8][kind as usize - 1];
            Ok((Value::Int(signed(slice(bytes, at, width)?)), width))
        }
        7 => Ok((Value::Other, 8)),
        8 => Ok((Value::Int(0), 0)),
        9 => Ok((Value::Int(1), 0)),
        10 | 11 => Err("ant.db record uses a reserved column type".to_string()),
        _ if kind % 2 == 0 => Ok((Value::Other, ((kind - 12) / 2) as usize)),
        _ => {
            let width = ((kind - 13) / 2) as usize;
            let text = std::str::from_utf8(slice(bytes, at, width)?).map_err(|_| "ant.db has text that is not UTF-8".to_string())?;
            Ok((Value::Text(text.to_string()), width))
        }
    }
}

// Two's complement, big-endian, sign taken from the first byte.
fn signed(bytes: &[u8]) -> i64 {
    let first = bytes.first().map_or(0, |&b| i64::from(b as i8));
    bytes.iter().skip(1).fold(first, |acc, &b| (acc << 8) | i64::from(b))
}

// Big-endian base-128 with up to nine bytes; the ninth byte keeps all eight bits.
fn varint(bytes: &[u8], at: usize) -> Result<(u64, usize), String> {
    let mut value: u64 = 0;
    for i in 0..8 {
        let byte = byte_at(bytes, at + i)?;
        value = (value << 7) | u64::from(byte & 0x7f);
        if byte & 0x80 == 0 {
            return Ok((value, i + 1));
        }
    }
    Ok(((value << 8) | u64::from(byte_at(bytes, at + 8)?), 9))
}

// The column names, in order, from a CREATE TABLE statement; constraint clauses are skipped.
fn columns(sql: &str) -> Vec<String> {
    let (Some(open), Some(close)) = (sql.find('('), sql.rfind(')')) else { return Vec::new() };
    let body = sql.get(open + 1..close).unwrap_or_default();
    let mut parts = Vec::new();
    let (mut depth, mut start) = (0i32, 0usize);
    for (i, c) in body.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(&body[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(&body[start..]);
    parts
        .into_iter()
        .filter_map(|part| {
            let word = part.split_whitespace().next()?.trim_matches('"');
            let upper = word.to_ascii_uppercase();
            (!RESERVED_WORDS.contains(&upper.as_str())).then(|| word.to_string())
        })
        .collect()
}

fn byte_at(bytes: &[u8], at: usize) -> Result<u8, String> {
    bytes.get(at).copied().ok_or_else(|| "ant.db is cut short".to_string())
}

fn slice(bytes: &[u8], at: usize, width: usize) -> Result<&[u8], String> {
    bytes.get(at..at + width).ok_or_else(|| "ant.db is cut short".to_string())
}

fn be_u16(bytes: &[u8], at: usize) -> Result<u16, String> {
    let b = slice(bytes, at, 2)?;
    Ok(u16::from_be_bytes([b[0], b[1]]))
}

fn be_u32(bytes: &[u8], at: usize) -> Result<u32, String> {
    let b = slice(bytes, at, 4)?;
    Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn varints_are_big_endian_base_128() {
        assert_eq!(varint(&[0x7f], 0), Ok((127, 1)));
        assert_eq!(varint(&[0x81, 0x00], 0), Ok((128, 2)));
    }

    #[test]
    fn a_file_that_is_not_sqlite_is_refused() {
        assert!(Database::open(&[b'x'; 128]).is_err());
    }

    #[test]
    fn column_names_skip_constraints_and_nested_commas() {
        let sql = "CREATE TABLE t (id INTEGER PRIMARY KEY, a TEXT, b TEXT, UNIQUE (a, b))";
        assert_eq!(columns(sql), vec!["id", "a", "b"]);
    }
}
