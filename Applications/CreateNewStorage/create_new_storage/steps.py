# The three wizard pages: drive, name, and review of the structure to create.
import os
import threading
import urllib.request
import tkinter as tk
from tkinter import ttk, filedialog

import yaml

from ant_core.theme import BG, TEXT, BUTTON, ACCENT, INPUT_BG, TREE_BG
from ant_core.shortcuts import IS_WINDOWS

from .config import TREE_YAML_URL
from .storage import get_available_drives
from .tree_preview import build_tree_widget


class StepsMixin:
    def show_step_1(self):
        self.current_step = 1
        self.update_subtitle("Step 1 of 3: Select Drive")
        self.clear_content()

        self.back_btn.config(state="disabled")
        self.next_btn.config(text="Next →", command=self.go_next)

        if IS_WINDOWS:
            drives = get_available_drives()

            tk.Label(
                self.content_frame,
                text="Select the drive where your ANT Storage will be created:",
                font=("Segoe UI", 10),
                bg=BG, fg=TEXT, justify="left"
            ).pack(anchor="w", pady=(0, 20))

            drive_frame = tk.Frame(self.content_frame, bg=BG)
            drive_frame.pack(fill="x", pady=10)

            tk.Label(
                drive_frame,
                text="Available drives:",
                font=("Segoe UI", 10),
                bg=BG, fg=TEXT
            ).pack(side="left", padx=(0, 10))

            self.drive_combo = ttk.Combobox(
                drive_frame,
                textvariable=self.selected_drive,
                values=drives,
                state="readonly",
                font=("Segoe UI", 10),
                foreground="#000000",
                width=10
            )
            self.drive_combo.pack(side="left")


            if drives:
                default = next((d for d in drives if not d.startswith("C")), drives[0])
                self.selected_drive.set(default)

            # Warning frame (hidden unless C: selected)
            warning_frame = tk.Frame(self.content_frame, bg="#3d2a1f", bd=1, relief="solid")
            tk.Label(
                warning_frame,
                text="⚠️  WARNING - Data Integrity Risk",
                font=("Segoe UI", 10, "bold"),
                bg="#3d2a1f", fg=ACCENT
            ).pack(anchor="w", padx=10, pady=(10, 5))
            tk.Label(
                warning_frame,
                text=(
                    "Storing your files on C: is not recommended:\n\n"
                    "• Windows Updates may wipe your data.\n"
                    "• C: is typically slower and more fragmented.\n"
                    "• ANT is designed for be portability on external drives."
                ),
                font=("Segoe UI", 9),
                bg="#3d2a1f", fg="#d4d4d4", justify="left"
            ).pack(anchor="w", padx=10, pady=(0, 10))

            # Show the warning only while C: is selected
            def update_warning(event=None):
                if self.selected_drive.get().upper().startswith("C"):
                    warning_frame.pack(fill="x", pady=(30, 0), padx=10)
                else:
                    warning_frame.pack_forget()

            self.drive_combo.bind("<<ComboboxSelected>>", update_warning)
            update_warning()

        else:
            # Unix: Entry + Browse button
            tk.Label(
                self.content_frame,
                text="Enter the base path where the storage will be created:",
                font=("Segoe UI", 10),
                bg=BG, fg=TEXT, justify="left"
            ).pack(anchor="w", pady=(0, 10))
            tk.Label(
                self.content_frame,
                text="Example: /home/user  or  /Volumes/MyDrive",
                font=("Segoe UI", 9),
                bg=BG, fg="#888888", justify="left"
            ).pack(anchor="w", pady=(0, 20))
            
            path_frame = tk.Frame(self.content_frame, bg=BG)
            path_frame.pack(fill="x", pady=10)
            
            self.path_entry = tk.Entry(
                path_frame,
                textvariable=self.selected_drive,
                font=("Segoe UI", 10),
                bg=INPUT_BG, fg=TEXT,
                insertbackground=TEXT, bd=0, relief="flat"
            )
            self.path_entry.pack(side="left", fill="x", expand=True, ipady=8, padx=(0, 10))
            
            tk.Button(
                path_frame,
                text="Browse...",
                font=("Segoe UI", 9),
                bg=INPUT_BG, fg=TEXT,
                activebackground="#3a3a3a", activeforeground=TEXT,
                bd=0, padx=15, pady=8, cursor="hand2",
                command=self.browse_directory
            ).pack(side="right")
            
            # Set default
            self.selected_drive.set(os.path.expanduser("~"))

    def browse_directory(self):
        path = filedialog.askdirectory(
            title="Select Base Directory",
            initialdir=self.selected_drive.get() or os.path.expanduser("~")
        )
        if path:
            self.selected_drive.set(path)

    def show_step_2(self):
        self.current_step = 2
        self.update_subtitle("Step 2 of 3: Name Your Storage")
        self.clear_content()
        
        self.back_btn.config(state="normal")
        self.next_btn.config(text="Next →", command=self.go_next)
        
        tk.Label(
            self.content_frame,
            text="Choose a descriptive name for your ANT Storage:",
            font=("Segoe UI", 10),
            bg=BG, fg=TEXT, justify="left"
        ).pack(anchor="w", pady=(0, 10))
        tk.Label(
            self.content_frame,
            text="Examples: My_Files   Home_Files   Mike_Storage",
            font=("Segoe UI", 9),
            bg=BG, fg="#888888", justify="left"
        ).pack(anchor="w", pady=(0, 20))
        
        # Name entry
        name_frame = tk.Frame(self.content_frame, bg=BG)
        name_frame.pack(fill="x", pady=10)
        
        tk.Label(
            name_frame,
            text="Storage name:",
            font=("Segoe UI", 10),
            bg=BG, fg=TEXT
        ).pack(anchor="w", pady=(0, 5))
        
        name_entry = tk.Entry(
            name_frame,
            textvariable=self.storage_name,
            font=("Segoe UI", 11),
            bg=INPUT_BG, fg=TEXT,
            insertbackground=TEXT, bd=0, relief="flat"
        )
        name_entry.pack(fill="x", ipady=10)
        name_entry.focus()
        
        preview_frame = tk.Frame(self.content_frame, bg="#1a2a1f", bd=1, relief="solid")
        preview_frame.pack(fill="x", pady=(20, 0), padx=10)
        
        tk.Label(
            preview_frame,
            text="ℹ️ The wizard will automatically adapt it to a valid name format (Pascal_Snake_Case):",
            font=("Segoe UI", 9),
            bg="#1a2a1f", fg="#b0b0b0"
        ).pack(anchor="w", padx=10, pady=(10, 5))
        
        tk.Label(
            preview_frame,
            textvariable=self.sanitized_name,
            font=("Consolas", 10, "bold"),
            bg="#1a2a1f", fg=BUTTON
        ).pack(anchor="w", padx=10, pady=(0, 10))

    def show_step_3(self):
        self.current_step = 3
        self.update_subtitle("Step 3 of 3: Review & Confirm")
        self.clear_content()
        
        self.back_btn.config(state="normal")
        self.next_btn.config(text="⏳ Loading...", state="disabled", command=self.create_storage)
        
        # Calculate full path
        drive = self.selected_drive.get()
        name  = self.sanitized_name.get()
        if IS_WINDOWS:
            self.full_path = os.path.join(drive + "\\", name)
        else:
            self.full_path = os.path.join(drive, name)
        
        # Path info
        info_frame = tk.Frame(self.content_frame, bg=INPUT_BG, bd=1, relief="solid")
        info_frame.pack(fill="x", pady=(0, 20))
        
        tk.Label(
            info_frame,
            text=f"Path | {self.full_path}",
            font=("Segoe UI", 9, "bold"),
            bg=INPUT_BG, fg=BUTTON, anchor="w"
        ).pack(fill="x", padx=10, pady=(2, 8))
        
        tk.Label(
            self.content_frame,
            text="Folder structure that will be created:",
            font=("Segoe UI", 10, "bold"),
            bg=BG, fg=TEXT
        ).pack(anchor="w", pady=(0, 10))
        
        tree_area = tk.Frame(self.content_frame, bg=BG)
        tree_area.pack(fill="both", expand=True)
        
        # Loading placeholder
        loading = tk.Frame(tree_area, bg=TREE_BG)
        loading.pack(fill="both", expand=True)
        tk.Label(
            loading, text="⏳  Loading preview from GitHub...",
            font=("Segoe UI", 10), bg=TREE_BG, fg="#888888"
        ).pack(expand=True)

        def _fetch():
            # Use cache if already fetched in this session
            if self.yaml_content is not None:
                self.root.after(0, lambda: _render(True))
                return
            try:
                req = urllib.request.Request(
                    TREE_YAML_URL, headers={"User-Agent": "ANT-Setup-Wizard"})
                with urllib.request.urlopen(req, timeout=10) as r:
                    raw = r.read().decode('utf-8')
                self.yaml_content = raw
                self.parsed_tree  = yaml.safe_load(raw)
                self.root.after(0, lambda: _render(True))
            except Exception as exc:
                self.root.after(0, lambda e=str(exc): _render(False, e))

        def _render(ok: bool, error: str = None):
            loading.destroy()

            if not ok:
                err_frame = tk.Frame(tree_area, bg=TREE_BG)
                err_frame.pack(fill="both", expand=True)
                tk.Label(
                    err_frame,
                    text=(f"⚠ Could not load preview\n{error}\n\n"
                          "Review your internet connection before to continue."),
                    font=("Segoe UI", 9), bg=TREE_BG, fg=ACCENT, justify="center"
                ).pack(expand=True, padx=20)
                self.next_btn.config(text="Create", state="normal")
                return

            scrollbar = tk.Scrollbar(tree_area, bg=INPUT_BG)
            scrollbar.pack(side="right", fill="y")

            tv = ttk.Treeview(tree_area, selectmode="none",
                              yscrollcommand=scrollbar.set, show="tree")
            tv.pack(side="left", fill="both", expand=True)
            scrollbar.config(command=tv.yview)

            root_node = tv.insert("", "end", text=f"📁 {name}/", open=True)
            build_tree_widget(self.parsed_tree, root_node, tv)

            self.next_btn.config(text="Create", state="normal")

        threading.Thread(target=_fetch, daemon=True).start()
