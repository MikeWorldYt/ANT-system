// Restarts Claude Desktop for the user, since it loads MCP servers only on start and closing its window only hides it in the tray.
use super::{claude_desktop, internal, AiClient};
use crate::domain::error::R;

// Only Claude Desktop's own exe: Claude Code's CLI is also claude.exe and must keep running.
#[cfg(windows)]
const STOP: &str = r"$p = Get-Process -Name Claude -ErrorAction SilentlyContinue | Where-Object { $_.Path -match '\\WindowsApps\\Claude_|\\AnthropicClaude\\' }
if ($p) { $p | Stop-Process -Force; $p | Wait-Process -Timeout 15 -ErrorAction SilentlyContinue }";

#[cfg(windows)]
const START: &str = r"$a = (Get-StartApps | Where-Object { $_.Name -eq 'Claude' } | Select-Object -First 1).AppID
if (-not $a) { throw 'Claude Desktop was not found in the Start menu' }
Start-Process explorer.exe ('shell:AppsFolder\' + $a)";

#[cfg(windows)]
fn powershell(script: &str) -> R<()> {
    crate::platform::windows::powershell::run_powershell(script, &[], None, std::time::Duration::from_secs(25)).map(|_| ()).map_err(internal)
}

#[cfg(windows)]
pub(super) fn restart_claude_desktop() -> R<AiClient> {
    let was_connected = claude_desktop::status(false)?.connected;
    powershell(STOP)?;
    // Claude may save its settings while closing; put the entry back if that dropped it.
    if was_connected && !claude_desktop::status(false)?.connected {
        claude_desktop::connect()?;
    }
    powershell(START)?;
    claude_desktop::status(false)
}

#[cfg(not(windows))]
pub(super) fn restart_claude_desktop() -> R<AiClient> {
    Err(internal("Restarting Claude Desktop is only supported on Windows"))
}
