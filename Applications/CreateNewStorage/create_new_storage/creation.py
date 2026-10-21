# Creates the storage on a worker thread while a progress window shows the log.
import os
import http.client
import queue
import threading
import urllib.request
import tkinter as tk
from tkinter import messagebox

import yaml

from ant_core.theme import BG, TEXT, BUTTON, ACCENT, ERROR, TREE_BG
from ant_core.shortcuts import IS_WINDOWS

from .config import DOCS_URL, README_URL, README_NAME, TREE_YAML_NAME, FOLDER_MANAGER_URL, FOLDER_MANAGER_NAME
from .storage import (
    make_dirs_tracked, rollback_created, write_file_atomic, download_tree_yaml,
    process_yaml_tree, download_core_files,
)

POLL_MS = 50


class CreationFailed(Exception):
    def __init__(self, title: str, message: str):
        super().__init__(message)
        self.title = title


class CreationMixin:
    def create_storage(self):
        if os.path.exists(self.full_path):
            result = messagebox.askyesno(
                "Path Exists",
                f"A folder named '{self.sanitized_name.get()}' already exists.\n\n"
                "Creating inside it may merge or overwrite content.\n\n"
                "Continue anyway?"
            )
            if not result:
                return

        self.back_btn.config(state="disabled")
        self.next_btn.config(state="disabled")
        progress_win, write_log = self._open_progress_window()

        # Tk is not thread-safe: the worker only queues events, the UI thread applies them
        events: queue.Queue = queue.Queue()

        def worker():
            try:
                readme_dest = self._creation_sequence(lambda msg, color=BUTTON: events.put(("log", msg, color)))
                events.put(("done", readme_dest))
            except CreationFailed as e:
                events.put(("failed", e.title, str(e)))
            except Exception as e:
                events.put(("log", f"\n❌ ERROR: {e}", ERROR))
                events.put(("failed", "Error", f"Failed to create storage:\n{e}"))

        def poll():
            while True:
                try:
                    event = events.get_nowait()
                except queue.Empty:
                    break
                if event[0] == "log":
                    write_log(event[1], event[2])
                elif event[0] == "done":
                    self.readme_path = event[1]
                    progress_win.destroy()
                    self._show_completion()
                    return
                else:
                    progress_win.grab_release()
                    progress_win.protocol("WM_DELETE_WINDOW", progress_win.destroy)
                    self.back_btn.config(state="normal")
                    self.next_btn.config(state="normal")
                    messagebox.showerror(event[1], event[2], parent=progress_win)
                    return
            self.root.after(POLL_MS, poll)

        threading.Thread(target=worker, daemon=True).start()
        self.root.after(POLL_MS, poll)

    def _open_progress_window(self):
        progress_win = tk.Toplevel(self.root)
        progress_win.title("Creating Storage...")
        progress_win.geometry("600x400")
        progress_win.resizable(False, False)
        progress_win.configure(bg=BG)
        progress_win.transient(self.root)
        progress_win.grab_set()
        # Closing mid-creation would orphan the worker; the window closes itself when done
        progress_win.protocol("WM_DELETE_WINDOW", lambda: None)

        progress_win.update_idletasks()
        x = (progress_win.winfo_screenwidth() // 2) - (600 // 2)
        y = (progress_win.winfo_screenheight() // 2) - (400 // 2)
        progress_win.geometry(f"+{x}+{y}")

        tk.Label(
            progress_win,
            text="🔨 Creating ANT Storage...",
            font=("Segoe UI", 12, "bold"),
            bg=BG, fg=TEXT
        ).pack(pady=(20, 10))

        log_frame = tk.Frame(progress_win, bg=BG)
        log_frame.pack(fill="both", expand=True, padx=20, pady=(0, 20))

        scrollbar = tk.Scrollbar(log_frame)
        scrollbar.pack(side="right", fill="y")

        log_text = tk.Text(
            log_frame,
            font=("Consolas", 9),
            bg=TREE_BG, fg=TEXT,
            insertbackground=TEXT,
            yscrollcommand=scrollbar.set,
            wrap="word", state="disabled"
        )
        log_text.pack(side="left", fill="both", expand=True)
        scrollbar.config(command=log_text.yview)

        log_text.tag_config("ok",     foreground=BUTTON)
        log_text.tag_config("accent", foreground=ACCENT)
        log_text.tag_config("err",    foreground=ERROR)
        log_text.tag_config("plain",  foreground=TEXT)
        tags = {BUTTON: "ok", ACCENT: "accent", ERROR: "err", TEXT: "plain"}

        def write_log(msg, color=BUTTON):
            log_text.config(state="normal")
            log_text.insert("end", msg + "\n", tags.get(color, "plain"))
            log_text.see("end")
            log_text.config(state="disabled")

        return progress_win, write_log

    def _creation_sequence(self, log) -> str:
        """Runs on the worker thread; returns the guide path or raises CreationFailed."""
        root_path    = self.full_path
        review_index = os.path.join(root_path, "998.Review")
        core_dir     = os.path.join(root_path, "999.Settings", "00.Core")
        created_dirs = []

        make_dirs_tracked(root_path, created_dirs)
        log("Wizard online\n\nGenerating root directory ...", ACCENT)
        log(f"✔ {root_path}")

        log("\nGenerating ANT Essentials ...", ACCENT)
        make_dirs_tracked(review_index, created_dirs)
        log(f"✔ {review_index}")
        make_dirs_tracked(core_dir, created_dirs)
        log(f"✔ {core_dir}")

        tree_dest = os.path.join(core_dir, TREE_YAML_NAME)
        if self.yaml_content:
            write_file_atomic(tree_dest, self.yaml_content)
            log(f"✔ {tree_dest}")
        else:
            try:
                self.yaml_content, self.parsed_tree = download_tree_yaml()
                write_file_atomic(tree_dest, self.yaml_content)
                log(f"✔ {tree_dest} (Downloaded)")
            except (OSError, ValueError, http.client.HTTPException, yaml.YAMLError) as e:
                log(f"✖ Error downloading Core YAML: {e}", ERROR)
                log("\nApplying roll-back...", ACCENT)
                remaining = rollback_created(created_dirs, log)
                leftover = ("\n\nThese folders could not be removed:\n" + "\n".join(remaining)) if remaining else ""
                raise CreationFailed(
                    "Error",
                    f"Failed to download essential file 'Tree.yaml':\n{e}\n\n"
                    "This file is essential to the creation of an ANT Storage.\n"
                    "Please check your internet connection and try again." + leftover
                ) from e

        log("\nGenerating an ANT Storage Template ...", ACCENT)
        process_yaml_tree(self.parsed_tree, root_path, review_index, log)

        log("\nDownloading ANT Core and utility files ...", ACCENT)
        readme_dest = os.path.join(root_path, README_NAME)
        try:
            urllib.request.urlretrieve(README_URL, readme_dest)
            log(f"✔ {readme_dest} (downloaded)")
        except OSError as e:
            with open(readme_dest, "w", encoding="utf-8") as f:
                f.write(
                    f"# ANT System — Get Started\n\n"
                    f"Could not download the guide automatically.\n"
                    f"Please visit:\n\n  {DOCS_URL}\n\nError: {e}\n"
                )
            log("⚠ Download failed. Placeholder created.", ACCENT)

        if IS_WINDOWS:
            fm_dest = os.path.join(root_path, FOLDER_MANAGER_NAME)
            try:
                urllib.request.urlretrieve(FOLDER_MANAGER_URL, fm_dest)
                self.folder_manager_path = fm_dest
                log(f"✔ {fm_dest} (downloaded)")
            except OSError as e:
                log(f"⚠ FolderManager download failed: {e}", ACCENT)

        try:
            download_core_files(core_dir, log)
        except (OSError, ValueError) as e:
            log(f"⚠ Core files download failed: {e}", ACCENT)

        log("\n" + "═" * 50, TEXT)
        log("🐜 Congrats, ANT Storage successfully created!", BUTTON)
        log("═" * 50 + "\n", TEXT)
        return readme_dest
