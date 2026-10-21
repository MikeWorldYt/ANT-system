# Creates the storage while a progress window shows the log.
import os
import urllib.request
import tkinter as tk
from tkinter import messagebox

import yaml

from ant_core.theme import BG, TEXT, BUTTON, ACCENT, ERROR, TREE_BG
from ant_core.shortcuts import IS_WINDOWS

from .config import DOCS_URL, README_URL, README_NAME, TREE_YAML_URL, TREE_YAML_NAME, FOLDER_MANAGER_URL, FOLDER_MANAGER_NAME
from .storage import make_dirs_tracked, rollback_created, process_yaml_tree, download_core_files


class CreationMixin:
    def create_storage(self):
        # Check if path already exists
        if os.path.exists(self.full_path):
            result = messagebox.askyesno(
                "Path Exists",
                f"A folder named '{self.sanitized_name.get()}' already exists.\n\n"
                "Creating inside it may merge or overwrite content.\n\n"
                "Continue anyway?"
            )
            if not result:
                return
        
        # Create progress window
        self.show_progress_window()

    def show_progress_window(self):
        """Show modal progress window during creation."""
        progress_win = tk.Toplevel(self.root)
        progress_win.title("Creating Storage...")
        progress_win.geometry("600x400")
        progress_win.resizable(False, False)
        progress_win.configure(bg=BG)
        progress_win.transient(self.root)
        progress_win.grab_set()
        
        # Center window
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
        
        # Log text widget
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

        # Fixed multi-color log: each color maps to a stable tag
        log_text.tag_config("ok",     foreground=BUTTON)
        log_text.tag_config("accent", foreground=ACCENT)
        log_text.tag_config("err",    foreground=ERROR)
        log_text.tag_config("plain",  foreground=TEXT)
        _TAG = {BUTTON: "ok", ACCENT: "accent", ERROR: "err", TEXT: "plain"}
        
        def log(msg, color=BUTTON):
            log_text.config(state="normal")
            log_text.insert("end", msg + "\n", _TAG.get(color, "plain"))
            log_text.see("end")
            log_text.config(state="disabled")
            progress_win.update()
        
        # ─────────────────────────────────────────────────────────
        # CREATION SEQUENCE
        # ─────────────────────────────────────────────────────────
        try:
            root_path    = self.full_path
            review_index = os.path.join(root_path, "998.Review")
            settings_dir = os.path.join(root_path, "999.Settings")
            core_dir     = os.path.join(settings_dir, "00.Core")

            created_dirs = []

            # ── Phase 1: Root ─────────────────────────────────────
            make_dirs_tracked(root_path, created_dirs)
            log(f"Wizard online\n\nGenerating root directory ...", ACCENT)
            log(f"✔ {root_path}")
            
            # ── Phase 2: Base CFG folders ─────────────────────────
            log(f"\nGenerating ANT Essentials ...", ACCENT)
            make_dirs_tracked(review_index, created_dirs)
            log(f"✔ {review_index}")
            make_dirs_tracked(core_dir, created_dirs)
            log(f"✔ {core_dir}")

            # ── Phase 3: Tree.yaml ────────────────────────────────
            tree_dest = os.path.join(core_dir, TREE_YAML_NAME)

            if self.yaml_content:
                with open(tree_dest, "w", encoding="utf-8") as f:
                    f.write(self.yaml_content)
                log(f"✔ {tree_dest}")
            else:
                try:
                    urllib.request.urlretrieve(TREE_YAML_URL, tree_dest)
                    with open(tree_dest, "r", encoding="utf-8") as f:
                        self.yaml_content = f.read()
                    self.parsed_tree = yaml.safe_load(self.yaml_content)
                    log(f"✔ {tree_dest} (Downloaded)")
                except Exception as e:
                    log(f"✖ Error downloading Core YAML: {e}", ERROR)
                    log("\nApplying roll-back...", ACCENT)
                    for path in reversed(created_dirs):
                        log(f"◄◄ {path} (Deleted)", ACCENT)
                    rollback_created(created_dirs)
                    messagebox.showerror(
                        "Error",
                        f"Failed to download essential file 'Tree.yaml':\n{e}\n\n"
                        "This file is essential to the creation of an ANT Storage.\n"
                        "Please check your internet connection and try again."
                    )
                    return

            # ── Phase 4: Build directory tree from YAML ───────────
            log(f"\nGenerating an ANT Storage Template ...", ACCENT)
            process_yaml_tree(self.parsed_tree, root_path, review_index, log)

            # ── Phase 5: README ───────────────────────────────────
            log(f"\nDownloading ANT Core and utility files ...", ACCENT)
            readme_dest = os.path.join(root_path, README_NAME)
            try:
                urllib.request.urlretrieve(README_URL, readme_dest)
                log(f"✔ {readme_dest} (downloaded)")
            except Exception as e:
                with open(readme_dest, "w", encoding="utf-8") as f:
                    f.write(
                        f"# ANT System — Get Started\n\n"
                        f"Could not download the guide automatically.\n"
                        f"Please visit:\n\n  {DOCS_URL}\n\nError: {e}\n"
                    )
                log(f"⚠ Download failed. Placeholder created.", ACCENT)

            # ── Phase 6: FolderManager.exe (Windows only) ─────────
            if IS_WINDOWS:
                fm_dest = os.path.join(root_path, FOLDER_MANAGER_NAME)
                try:
                    urllib.request.urlretrieve(FOLDER_MANAGER_URL, fm_dest)
                    self.folder_manager_path = fm_dest
                    log(f"✔ {fm_dest} (downloaded)")
                except Exception as e:
                    log(f"⚠ FolderManager download failed: {e}", ACCENT)

            # ── Phase 7: Core utility files ───────────────────────
            try:
                download_core_files(core_dir, log)
            except Exception as e:
                log(f"⚠ Core files download failed: {e}", ACCENT)

            # ── Done ──────────────────────────────────────────────
            log("\n" + "═" * 50, TEXT)
            log("🐜 Congrats, ANT Storage successfully created!", BUTTON)
            log("Please close this window and continue.", TEXT)
            log("═" * 50 + "\n", TEXT)

            self.readme_path = readme_dest
            self.root.after(100, lambda: self._show_completion(progress_win))
            
        except Exception as e:
            log(f"\n❌ ERROR: {str(e)}", ERROR)
            messagebox.showerror("Error", f"Failed to create storage:\n{str(e)}")
