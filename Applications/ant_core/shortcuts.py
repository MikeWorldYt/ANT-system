# Review-index shortcuts: their names and how they are created on each OS.
import os
import platform
import subprocess

IS_WINDOWS = platform.system() == "Windows"


def review_shortcut_name(owner_parts: list[str]) -> str:
    """998.Review shortcut name for the REV folder inside owner_parts[-1], e.g. 400.ACADEMIC/01.Math -> 401.Math."""
    if len(owner_parts) == 1:
        return owner_parts[0]
    if len(owner_parts) == 2:
        return owner_parts[0][0] + owner_parts[1]
    if len(owner_parts) == 3:
        return owner_parts[0][0] + owner_parts[1].split(".")[0] + "." + owner_parts[2]
    raise ValueError(f"No review shortcut below level 3: {'/'.join(owner_parts)}")


def create_shortcut(target_path: str, link_path_no_ext: str) -> str:
    """Create a .lnk on Windows or a symlink elsewhere; return the path created."""
    if not IS_WINDOWS:
        if not os.path.lexists(link_path_no_ext):
            os.symlink(target_path, link_path_no_ext)
        return link_path_no_ext

    link_path = link_path_no_ext + ".lnk"
    # Paths travel as env vars so quotes, $ or backticks in folder names can't break the script
    ps_cmd = (
        "$s = (New-Object -ComObject WScript.Shell).CreateShortcut($env:ANT_LNK_PATH); "
        "$s.TargetPath = $env:ANT_LNK_TARGET; "
        "$s.Save()"
    )
    result = subprocess.run(
        ["powershell", "-NoProfile", "-NonInteractive", "-Command", ps_cmd],
        env={**os.environ, "ANT_LNK_PATH": link_path, "ANT_LNK_TARGET": target_path},
        creationflags=subprocess.CREATE_NO_WINDOW,
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        raise OSError(f"Shortcut not created: {link_path}\n{result.stderr.strip()}")
    return link_path
