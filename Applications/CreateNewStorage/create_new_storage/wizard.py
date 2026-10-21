# Wizard window: shared layout, styles and navigation between the steps.
import os
import tkinter as tk
from tkinter import ttk, messagebox

from ant_core.theme import BG, TEXT, BUTTON, ACCENT, INPUT_BG, TREE_BG
from ant_core.shortcuts import IS_WINDOWS

from .config import ANT_VERSION
from .storage import sanitize_name
from .steps import StepsMixin
from .creation import CreationMixin
from .completion import CompletionMixin


class ANTWizard(StepsMixin, CreationMixin, CompletionMixin):
    def __init__(self, root):
        self.root = root
        self.root.title(f"ANT System Setup - New Storage Wizard {ANT_VERSION} ")
        self.root.geometry("750x550")
        self.root.resizable(False, False)
        self.root.configure(bg=BG)
        
        # Center window
        self.root.update_idletasks()
        x = (self.root.winfo_screenwidth() // 2) - (750 // 2)
        y = (self.root.winfo_screenheight() // 2) - (550 // 2)
        self.root.geometry(f"+{x}+{y}")
        
        # Data
        self.current_step   = 1
        self.selected_drive = tk.StringVar()
        self.storage_name   = tk.StringVar()
        self.sanitized_name = tk.StringVar()
        self.full_path      = ""
        self.yaml_content         = None   # cached raw YAML string
        self.parsed_tree          = None   # cached parsed dict
        self.readme_path          = ""
        self.folder_manager_path  = ""
        
        self.storage_name.trace("w", self.update_sanitized_name)
        
        # Setup styles
        self.setup_styles()
        
        # Main container
        self.main_frame = tk.Frame(self.root, bg=BG)
        self.main_frame.pack(fill="both", expand=True, padx=20, pady=20)
        
        # Header
        self.header_frame = tk.Frame(self.main_frame, bg=BG)
        self.header_frame.pack(fill="x", pady=(0, 20))
        
        self.title_label = tk.Label(
            self.header_frame,
            text="🐜 ANT System Setup",
            font=("Segoe UI", 16, "bold"),
            bg=BG,
            fg=TEXT
        )
        self.title_label.pack()
        
        self.subtitle_label = tk.Label(
            self.header_frame,
            text="Step 1 of 3: Select Drive",
            font=("Segoe UI", 10),
            bg=BG,
            fg=ACCENT
        )
        self.subtitle_label.pack()
        
        # Content frame (will swap between steps)
        self.content_frame = tk.Frame(self.main_frame, bg=BG)
        self.content_frame.pack(fill="both", expand=True)
        
        # Navigation frame
        self.nav_frame = tk.Frame(self.main_frame, bg=BG)
        self.nav_frame.pack(fill="x", pady=(20, 0))
        
        self.back_btn = tk.Button(
            self.nav_frame,
            text="← Back",
            font=("Segoe UI", 10),
            bg=INPUT_BG,
            fg=TEXT,
            activebackground="#3a3a3a",
            activeforeground=TEXT,
            bd=0,
            padx=20,
            pady=8,
            cursor="hand2",
            command=self.go_back
        )
        self.back_btn.pack(side="left")
        
        self.next_btn = tk.Button(
            self.nav_frame,
            text="Next →",
            font=("Segoe UI", 10, "bold"),
            bg=BUTTON,
            fg=TEXT,
            activebackground="#00a077",
            activeforeground=TEXT,
            bd=0,
            padx=20,
            pady=8,
            cursor="hand2",
            command=self.go_next
        )
        self.next_btn.pack(side="right")
        
        # Show first step
        self.show_step_1()

    def setup_styles(self):
        """Configure ttk styles for themed widgets."""
        style = ttk.Style()
        
        # Treeview style
        style.theme_use("default")
        style.configure("Treeview",
            background=TREE_BG,
            foreground=TEXT,
            fieldbackground=TREE_BG,
            borderwidth=0,
            font=("Consolas", 9)
        )
        style.configure("Treeview.Heading",
            background=INPUT_BG,
            foreground=TEXT,
            borderwidth=0,
            font=("Segoe UI", 10, "bold")
        )
        style.map("Treeview",
            background=[("selected", BUTTON)],
            foreground=[("selected", TEXT)]
        )
        
        # Combobox style
        style.configure("TCombobox",
            fieldbackground=INPUT_BG,
            background=INPUT_BG,
            foreground=TEXT,
            arrowcolor=TEXT,
            borderwidth=0
        )

    def clear_content(self):
        for widget in self.content_frame.winfo_children():
            widget.destroy()

    def update_subtitle(self, text):
        self.subtitle_label.config(text=text)

    def update_sanitized_name(self, *args):
        self.sanitized_name.set(sanitize_name(self.storage_name.get()))

    def go_back(self):
        if self.current_step == 2:
            self.show_step_1()
        elif self.current_step == 3:
            self.show_step_2()

    def go_next(self):
        if self.current_step == 1:
            # Validate drive selection
            drive = self.selected_drive.get()
            
            if not drive:
                messagebox.showerror("Error", "Please select a drive or enter a path.")
                return
            
            if not IS_WINDOWS and not os.path.isdir(drive):
                messagebox.showerror("Error", f"Path '{drive}' does not exist.")
                return
            
            # C: warning on Windows
            if IS_WINDOWS and drive.startswith("C:"):
                result = messagebox.askyesno(
                    "C: Drive Warning",
                    "You selected C: drive.\n\n"
                    "This is not recommended as Windows reinstalls may wipe your data.\n\n"
                    "Continue anyway?"
                )
                if not result:
                    return
            self.show_step_2()
        
        elif self.current_step == 2:
            if not self.sanitized_name.get():
                messagebox.showerror("Error", "Storage name cannot be empty.")
                return
            self.show_step_3()
