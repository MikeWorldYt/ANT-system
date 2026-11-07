// Resolves the Windows tools from the system folder so none is taken from the app folder or PATH.
use std::ffi::OsString;
use std::io;
use std::path::PathBuf;
use std::process::Command;

pub(crate) fn system_tool(name: &str) -> io::Result<PathBuf> {
    tool_path(std::env::var_os("SystemRoot"), name)
}

// Callers that must hand back a Command get one that cannot start when SystemRoot is unusable, instead of a bare name.
pub(crate) fn system_command(name: &str) -> Command {
    Command::new(system_tool(name).unwrap_or_else(|_| PathBuf::from(r"\\?\SystemRoot-is-unusable").join(name)))
}

fn tool_path(system_root: Option<OsString>, name: &str) -> io::Result<PathBuf> {
    let root = PathBuf::from(system_root.unwrap_or_default());
    if !root.is_absolute() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "SystemRoot is unset or not an absolute path, so the Windows tool is not started"));
    }
    // Explorer is the one tool that lives in the Windows folder itself.
    Ok(if name == "explorer.exe" { root.join(name) } else { root.join("System32").join(name) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tools_join_the_system_folder_and_explorer_the_windows_folder() {
        let root = || Some(OsString::from(r"C:\Windows"));
        assert_eq!(tool_path(root(), "attrib.exe").unwrap(), PathBuf::from(r"C:\Windows\System32\attrib.exe"));
        assert_eq!(tool_path(root(), r"WindowsPowerShell\v1.0\powershell.exe").unwrap(), PathBuf::from(r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe"));
        assert_eq!(tool_path(root(), "explorer.exe").unwrap(), PathBuf::from(r"C:\Windows\explorer.exe"));
    }

    #[test]
    fn an_unset_empty_or_relative_system_root_is_refused() {
        for root in [None, Some(OsString::new()), Some(OsString::from(r"Windows")), Some(OsString::from(r"..\Windows"))] {
            let err = tool_path(root.clone(), "attrib.exe").unwrap_err();
            assert!(err.to_string().contains("SystemRoot"), "{root:?}: {err}");
        }
    }

    #[test]
    fn the_real_system_folder_gives_an_absolute_path_ending_in_the_tool() {
        for name in ["attrib.exe", "rundll32.exe", "explorer.exe", "schtasks.exe", "curl.exe"] {
            let path = system_tool(name).unwrap();
            assert!(path.is_absolute(), "{path:?}");
            assert!(path.ends_with(name), "{path:?}");
        }
    }
}
