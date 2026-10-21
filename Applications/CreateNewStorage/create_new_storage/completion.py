# Last page: offers to launch FolderManager and open the guide.
import os
import platform
import subprocess
import tkinter as tk

from ant_core.theme import BG, TEXT, BUTTON, INPUT_BG
from ant_core.shortcuts import IS_WINDOWS


class CompletionMixin:
    def _show_completion(self, progress_win):


        self.clear_content()
        self.update_subtitle("Setup Complete")
        self.back_btn.config(state="disabled")

        # ── Two-column layout: logo | content ─────────────────────
        outer = tk.Frame(self.content_frame, bg=BG)
        outer.pack(fill="both", expand=True)

        # Left column — logo
        logo_col = tk.Frame(outer, bg=BG, width=150)
        logo_col.pack(side="left", fill="y", padx=(0, 25))
        logo_col.pack_propagate(False)

        tk.Label(logo_col, text="🐜", font=("Segoe UI", 52), bg=BG, fg=BUTTON
                 ).pack(expand=True)

        # Right column — content
        right = tk.Frame(outer, bg=BG)
        right.pack(side="left", fill="both", expand=True)

        tk.Label(right,
                 text="Completing the ANT System Setup Wizard",
                 font=("Segoe UI", 13, "bold"),
                 bg=BG, fg=TEXT, justify="left"
                 ).pack(anchor="w", pady=(5, 15))

        tk.Label(right,
                 text=f"Setup has finished creating your ANT Storage at: {self.full_path}",
                 font=("Segoe UI", 9),
                 bg=BG, fg="#b0b0b0"
                 ).pack(anchor="w")

        tk.Label(right,
                 text="Click Finish to exit the setup.",
                 font=("Segoe UI", 9),
                 bg=BG, fg="#b0b0b0"
                 ).pack(anchor="w", pady=(0, 25))

        # ── Checkboxes ────────────────────────────────────────────
        cb_fm_var     = tk.BooleanVar(value=True)
        cb_readme_var = tk.BooleanVar(value=False)

        fm_cb = tk.Checkbutton(
            right,
            text="Launch Folder Manager",
            variable=cb_fm_var,
            font=("Segoe UI", 10),
            bg=BG, fg=TEXT,
            activebackground=BG, activeforeground=TEXT,
            selectcolor=INPUT_BG,
            cursor="hand2"
        )
        fm_cb.pack(anchor="w", pady=(0, 8))

        # Disable FM checkbox if download failed
        if not self.folder_manager_path or not os.path.exists(self.folder_manager_path):
            cb_fm_var.set(False)
            fm_cb.config(state="disabled", fg="#666666")

        tk.Checkbutton(
            right,
            text="Open Get Started Guide",
            variable=cb_readme_var,
            font=("Segoe UI", 10),
            bg=BG, fg=TEXT,
            activebackground=BG, activeforeground=TEXT,
            selectcolor=INPUT_BG,
            cursor="hand2"
        ).pack(anchor="w")

        # ── Finish button ─────────────────────────────────────────
        self.next_btn.config(
            text="Finish",
            state="normal",
            command=lambda: self._finish(cb_fm_var.get(), cb_readme_var.get())
        )

    def _finish(self, launch_fm: bool, open_readme: bool):
        """Execute selected actions and close the wizard."""
        if launch_fm and self.folder_manager_path and os.path.exists(self.folder_manager_path):
            exe_dir = os.path.dirname(self.folder_manager_path)
            subprocess.Popen([self.folder_manager_path], cwd=exe_dir)

        if open_readme and self.readme_path and os.path.exists(self.readme_path):
            readme_dir = os.path.dirname(self.readme_path)
            if IS_WINDOWS:
                subprocess.Popen(["notepad.exe", self.readme_path], cwd=readme_dir)
            else:
                opener = "open" if platform.system() == "Darwin" else "xdg-open"
                subprocess.Popen([opener, self.readme_path], cwd=readme_dir)

        self.root.destroy()
