// The MCP side: reads the endpoint the app wrote, sends one request with its token and waits for the reply.
use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::path::Path;
use std::time::Duration;

use super::{endpoint, Reply, Request, UiCall, REPLY_TIMEOUT};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(1);

pub const NOT_RUNNING: &str = "Folder Manager is not running, so the ui_* tools have no window to move; open the app and call again. Every other tool works without it.";
const REFUSED: &str = "Folder Manager did not accept the request; if it was just restarted, call the tool again";

pub fn send(file: &Path, root: &Path, call: UiCall) -> Reply {
    let endpoint = endpoint::read(file).ok_or_else(|| NOT_RUNNING.to_string())?;
    // Only the port is taken from the file; the host is always this machine.
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, endpoint.port));
    // A refused connection is an app that closed without removing its file, for example after a crash.
    let stream = TcpStream::connect_timeout(&address, CONNECT_TIMEOUT).map_err(|_| NOT_RUNNING.to_string())?;
    let failed = |e: std::io::Error| format!("The UI bridge to Folder Manager failed: {e}");
    stream.set_read_timeout(Some(REPLY_TIMEOUT)).map_err(failed)?;
    stream.set_write_timeout(Some(REPLY_TIMEOUT)).map_err(failed)?;
    let text = serde_json::to_string(&Request { token: endpoint.token, root: root.to_path_buf(), call }).map_err(|e| e.to_string())?;
    let mut out = &stream;
    writeln!(out, "{text}").and_then(|()| out.flush()).map_err(failed)?;
    let mut line = Vec::new();
    BufReader::new(&stream).read_until(b'\n', &mut line).map_err(failed)?;
    if line.is_empty() {
        return Err(REFUSED.to_string());
    }
    serde_json::from_slice::<Reply>(&line).map_err(|e| format!("Folder Manager sent an unreadable reply: {e}"))?
}
