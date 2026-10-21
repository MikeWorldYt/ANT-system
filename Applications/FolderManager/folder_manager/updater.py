# Update check against the version published on main, and the dialog that starts FM_Updater.bat.
import os
import subprocess
import threading
import tkinter as tk
from tkinter import messagebox

from ant_core.theme import BG, TEXT, BUTTON, ACCENT, INPUT_BG, DISABLED_TEXT, BORDER, HOVER_BTN
from ant_core.version import parse_file_version, parse_version

from .config import VERSION_URL, EXE_DOWNLOAD_URL, UPDATER_SCRIPT, CURRENT_VERSION, SETTINGS_FOLDER


def fetch_latest_version() -> str | None:
    """Return the FileVersion published on main, or None when offline or unparsable."""
    import urllib.request, urllib.error
    req = urllib.request.Request(VERSION_URL, headers={"User-Agent": "FolderManager-Updater"})
    try:
        with urllib.request.urlopen(req, timeout=5) as resp:
            text = resp.read().decode("utf-8", errors="replace")
    except (urllib.error.URLError, TimeoutError, OSError):
        return None
    return parse_file_version(text)


def load_skipped_version(core_path: str) -> str:
    skip_file = os.path.join(core_path, ".skip_version")
    try:
        with open(skip_file, "r") as f:
            return f.read().strip()
    except OSError:
        return ""


def save_skipped_version(core_path: str, version: str):
    skip_file = os.path.join(core_path, ".skip_version")
    try:
        with open(skip_file, "w") as f:
            f.write(version)
    except OSError:
        # Skipping is only a preference: if it can't be stored, the dialog shows again next launch
        return


def launch_updater(core_path: str, exe_path: str):
    """Launch FM_Updater.bat and exit the app."""
    bat = os.path.join(core_path, UPDATER_SCRIPT)
    if not os.path.isfile(bat):
        messagebox.showerror(
            "Updater not found",
            f"Could not find {UPDATER_SCRIPT} in:\n{core_path}"
        )
        return False
    # cmd /s strips only the outer quote pair, so every quoted path survives spaces
    command = f'cmd /d /s /c ""{bat}" "{exe_path}" "{EXE_DOWNLOAD_URL}""'
    try:
        proc = subprocess.Popen(command, creationflags=subprocess.CREATE_NO_WINDOW)
    except OSError as e:
        messagebox.showerror("Update failed", f"Could not start the updater:\n{e}")
        return False
    try:
        # The updater keeps running while it waits for this window to close; an early exit means it failed
        code = proc.wait(timeout=1.5)
    except subprocess.TimeoutExpired:
        return True
    messagebox.showerror("Update failed", f"The updater stopped right away (exit code {code}).\nFolderManager stays open.")
    return False


class UpdateDialog(tk.Toplevel):
    """Modal dialog shown when a new version is available."""

    def __init__(self, parent: tk.Tk, latest: str, core_path: str, exe_path: str):
        super().__init__(parent)
        self.parent    = parent
        self.latest    = latest
        self.core_path = core_path
        self.exe_path  = exe_path
        self.result    = None   # "update" | "skip" | "later"

        self.title("Update Available")
        self.resizable(False, False)
        self.configure(bg=BG)
        self.grab_set()         # modal

        # Center over parent
        self.update_idletasks()
        pw, ph = parent.winfo_width(), parent.winfo_height()
        px, py = parent.winfo_x(), parent.winfo_y()
        w, h   = 360, 190
        self.geometry(f"{w}x{h}+{px + (pw-w)//2}+{py + (ph-h)//2}")

        # ── Content ──────────────────────────────────────────────
        tk.Label(self, text="🆕  New version available",
                 bg=BG, fg=ACCENT, font=("Segoe UI", 11, "bold"),
                 pady=14).pack()

        tk.Label(self,
                 text=f"Current:  {CURRENT_VERSION}\nLatest:     {latest}",
                 bg=BG, fg=TEXT, font=("Segoe UI", 10),
                 justify=tk.LEFT).pack(padx=24, anchor="w")

        tk.Label(self,
                 text="The app will close to apply the update.",
                 bg=BG, fg=DISABLED_TEXT, font=("Segoe UI", 8),
                 pady=6).pack()

        btn_frame = tk.Frame(self, bg=BG)
        btn_frame.pack(fill=tk.X, padx=16, pady=(4, 14))

        btn_cfg = dict(font=("Segoe UI", 9, "bold"), bd=0, relief=tk.FLAT,
                       pady=7, cursor="hand2")

        tk.Button(btn_frame, text="Update", bg=BUTTON, fg=BG,
                  activebackground=HOVER_BTN, activeforeground=BG,
                  command=self._on_update, **btn_cfg).pack(
                  side=tk.LEFT, expand=True, fill=tk.X, padx=(0, 4))

        tk.Button(btn_frame, text="Skip version", bg=BORDER, fg=TEXT,
                  activebackground=INPUT_BG, activeforeground=TEXT,
                  command=self._on_skip, **btn_cfg).pack(
                  side=tk.LEFT, expand=True, fill=tk.X, padx=(0, 4))

        tk.Button(btn_frame, text="Remind me later", bg=INPUT_BG, fg=DISABLED_TEXT,
                  activebackground=BORDER, activeforeground=TEXT,
                  command=self._on_later, **btn_cfg).pack(
                  side=tk.LEFT, expand=True, fill=tk.X)

        self.protocol("WM_DELETE_WINDOW", self._on_later)
        self.wait_window()

    def _on_update(self):
        self.result = "update"
        self.destroy()
        if launch_updater(self.core_path, self.exe_path):
            self.parent.destroy()

    def _on_skip(self):
        save_skipped_version(self.core_path, self.latest)
        self.result = "skip"
        self.destroy()

    def _on_later(self):
        self.result = "later"
        self.destroy()


def check_for_updates(parent: tk.Tk, root_path: str, exe_path: str):
    """Run version check in background thread; show dialog on main thread if needed."""
    import threading

    core_path = os.path.join(root_path, SETTINGS_FOLDER, "00.Core")

    def _worker():
        latest = fetch_latest_version()
        if not latest:
            return
        if parse_version(latest) <= parse_version(CURRENT_VERSION):
            return
        skipped = load_skipped_version(core_path)
        if latest == skipped:
            return
        # Schedule dialog on main thread
        parent.after(0, lambda: UpdateDialog(parent, latest, core_path, exe_path))

    t = threading.Thread(target=_worker, daemon=True)
    t.start()
