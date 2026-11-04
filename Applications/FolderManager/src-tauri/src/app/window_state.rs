// Remembers the window size between launches in a file outside the Storage and its database.
use serde::Deserialize;
use tauri::{LogicalSize, Manager, WebviewWindow};

const FILE: &str = "window-state.json";
const MIN_SIZE: (f64, f64) = (640.0, 400.0);
const MAX_SIZE: f64 = 8000.0;

#[derive(Debug, PartialEq, Deserialize)]
struct State {
    width: f64,
    height: f64,
    #[serde(default)]
    maximized: bool,
}

fn parse(text: &str) -> Option<State> {
    let mut state: State = serde_yaml::from_str(text).ok()?;
    if !state.width.is_finite() || !state.height.is_finite() {
        return None;
    }
    state.width = state.width.clamp(MIN_SIZE.0, MAX_SIZE);
    state.height = state.height.clamp(MIN_SIZE.1, MAX_SIZE);
    Some(state)
}

fn render(state: &State) -> String {
    format!("{{\"width\": {:.0}, \"height\": {:.0}, \"maximized\": {}}}\n", state.width, state.height, state.maximized)
}

fn file(window: &WebviewWindow) -> Result<std::path::PathBuf, String> {
    let dir = window.app_handle().path().app_local_data_dir().map_err(|e| e.to_string())?;
    Ok(dir.join(FILE))
}

pub fn restore(window: &WebviewWindow) {
    let Ok(path) = file(window) else { return };
    let Some(state) = std::fs::read_to_string(path).ok().and_then(|text| parse(&text)) else { return };
    if let Err(e) = window.set_size(LogicalSize::new(state.width, state.height)) {
        eprintln!("Could not restore the window size: {e}");
    }
    if state.maximized {
        if let Err(e) = window.maximize() {
            eprintln!("Could not restore the maximized window: {e}");
        }
    }
}

pub fn save(window: &WebviewWindow) {
    if window.is_minimized().unwrap_or(false) {
        return;
    }
    let result = (|| -> Result<(), String> {
        let path = file(window)?;
        let maximized = window.is_maximized().map_err(|e| e.to_string())?;
        let previous = std::fs::read_to_string(&path).ok().and_then(|text| parse(&text));
        // A maximized window reports the screen size, so the size before maximizing is kept.
        let (width, height) = match (maximized, previous) {
            (true, Some(prev)) => (prev.width, prev.height),
            _ => {
                let size = window.inner_size().map_err(|e| e.to_string())?;
                let scale = window.scale_factor().map_err(|e| e.to_string())?;
                (f64::from(size.width) / scale, f64::from(size.height) / scale)
            }
        };
        let dir = path.parent().ok_or("no data folder")?;
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        std::fs::write(&path, render(&State { width, height, maximized })).map_err(|e| e.to_string())
    })();
    if let Err(e) = result {
        eprintln!("Could not save the window size: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_saved_state_reads_back_and_is_clamped() {
        let state = State { width: 1000.0, height: 640.0, maximized: true };
        assert_eq!(parse(&render(&state)), Some(state));
        let tiny = parse(r#"{"width": 10, "height": 20}"#).unwrap();
        assert_eq!((tiny.width, tiny.height, tiny.maximized), (MIN_SIZE.0, MIN_SIZE.1, false));
        assert_eq!(parse(r#"{"width": 99999, "height": 700}"#).unwrap().width, MAX_SIZE);
    }

    #[test]
    fn garbled_state_is_ignored() {
        assert_eq!(parse("not a state"), None);
        assert_eq!(parse(r#"{"width": "wide", "height": 700}"#), None);
        assert_eq!(parse(r#"{"width": .nan, "height": 700}"#), None);
    }
}
