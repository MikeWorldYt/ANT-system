// The app side: one connection at a time on 127.0.0.1, dropped unanswered without the session token, else checked against the open storage.
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::time::Duration;

use serde_json::Value;

use super::{Reply, Request, UiCall};
use crate::services::watch::storage_key;

const MAX_REQUEST: u64 = 64 * 1024;
const READ_TIMEOUT: Duration = Duration::from_secs(2);

pub fn bind() -> std::io::Result<(TcpListener, u16)> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
    let port = listener.local_addr()?.port();
    Ok((listener, port))
}

pub fn serve(listener: TcpListener, token: String, open: impl Fn() -> Option<PathBuf>, run: impl Fn(UiCall) -> Reply) {
    for stream in listener.incoming() {
        if let Err(e) = stream.and_then(|s| handle(&s, &token, &open, &run)) {
            eprintln!("UI bridge: {e}");
        }
    }
}

fn handle(stream: &TcpStream, token: &str, open: &impl Fn() -> Option<PathBuf>, run: &impl Fn(UiCall) -> Reply) -> std::io::Result<()> {
    stream.set_read_timeout(Some(READ_TIMEOUT))?;
    stream.set_write_timeout(Some(READ_TIMEOUT))?;
    let mut line = Vec::new();
    BufReader::new(stream.take(MAX_REQUEST)).read_until(b'\n', &mut line)?;
    let Some(request) = parse(&line, token) else { return Ok(()) };
    let reply = request.and_then(|r| answer(r, open(), run));
    let mut out = stream;
    writeln!(out, "{}", serde_json::to_string(&reply)?)?;
    out.flush()
}

// None means ignored: a request without the session token gets nothing read past it and no answer.
pub(super) fn parse(line: &[u8], token: &str) -> Option<Result<Request, String>> {
    let value: Value = serde_json::from_slice(line).ok()?;
    let sent = value.get("token")?.as_str()?;
    if !same(sent.as_bytes(), token.as_bytes()) {
        return None;
    }
    Some(serde_json::from_value(value).map_err(|e| format!("Invalid UI request: {e}")))
}

// Every byte is compared whatever the first mismatch, so the time taken tells nothing about the token.
fn same(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

pub(super) fn answer(request: Request, open: Option<PathBuf>, run: &impl Fn(UiCall) -> Reply) -> Reply {
    let open = open.ok_or_else(|| "Folder Manager has no storage open".to_string())?;
    if storage_key(&request.root) != storage_key(&open) {
        return Err(format!("Folder Manager has another storage open ({}), so it will not move its window for {}", open.display(), request.root.display()));
    }
    run(request.call)
}
