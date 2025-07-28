"""
╔══════════════════════════════════════════════════════════════════╗
║              ANT SYSTEM — Create New Storage (GUI)               ║
║          Archive Nesting Technique on the System                 ║
║                                                                  ║
║  GUI wizard to initialize a new ANT root storage with full       ║
║  folder structure, 98.Review sandboxes, and shortcuts.           ║
║                                                                  ║
║  Docs:   https://ant-system.vercel.app                          ║
║  GitHub: https://github.com/MikeWorldYt/ANT-system              ║
╚══════════════════════════════════════════════════════════════════╝
"""

import os
import sys
import platform
import string
import re
import urllib.request
import tkinter as tk
from tkinter import ttk, messagebox, filedialog
import subprocess

# ─────────────────────────────────────────────────────────────────
#  CONSTANTS
# ─────────────────────────────────────────────────────────────────

DOCS_URL = "https://ant-system.vercel.app/en/start-here/migrating-plan"
README_URL = "https://raw.githubusercontent.com/MikeWorldYt/ANT-system/main/Migration/01.Get_Started.md"
README_NAME = "01.Get_Started.md"

ANT_VERSION = "v1.0"

# Main Level-1 structure
LEVEL1 = [
    "100.PERSONAL",
    "200.WORK",
    "300.PROJECTS",
    "400.ACADEMIC",
    "500.HOBBIES",
    "900.Resources",
    "998.Review",
    "999.Settings",
]

# These Level-1 folders will each get a 98.Review sandbox inside
REVIEW_TARGETS = [
    "100.PERSONAL",
    "200.WORK",
    "300.PROJECTS",
    "400.ACADEMIC",
    "500.HOBBIES",
]

# ─────────────────────────────────────────────────────────────────
#  COLOR THEME
# ─────────────────────────────────────────────────────────────────

BG = "#202020"
TEXT = "#FFFFFF"
BUTTON = "#00b386"
ACCENT = "#ff8b40"
INPUT_BG = "#2b2b2b"
TREE_BG = "#1a1a1a"

# ─────────────────────────────────────────────────────────────────
#  HELPERS
# ─────────────────────────────────────────────────────────────────

IS_WINDOWS = platform.system() == "Windows"

def sanitize_name(raw: str) -> str:
    """Sanitize and format name to Pascal_Snake_Case."""
    name = raw.strip()
    name = re.sub(r'[\s_]+', '_', name)
    name = re.sub(r'[\\/:*?"<>|]', '', name)
    parts = name.split('_')
    parts = [p.capitalize() for p in parts if p]
    # Unir con underscore
    return '_'.join(parts)

def get_available_drives() -> list:
    """Return available drive letters on Windows, or ['/'] on Unix."""
    if not IS_WINDOWS:
        return ["/"]
    drives = []
    for letter in string.ascii_uppercase:
        drive = f"{letter}:\\"
        if os.path.exists(drive):
            drives.append(f"{letter}:")
    return drives


def create_shortcut_windows(target_path: str, shortcut_path: str):
    ps_cmd = (
        f"$ws = New-Object -ComObject WScript.Shell; "
        f"$s = $ws.CreateShortcut('{shortcut_path}'); "
        f"$s.TargetPath = '{target_path}'; "
        f"$s.Save()"
    )
    os.system(f"powershell -NoProfile -Command \"{ps_cmd}\"")


def create_symlink_unix(target_path: str, link_path: str):
    """Create a symbolic link on Unix systems."""
    if not os.path.exists(link_path):
        os.symlink(target_path, link_path)


def open_folder(path: str):
    """Open folder in file explorer."""
    if IS_WINDOWS:
        os.startfile(path)
    elif platform.system() == "Darwin":  # macOS
        subprocess.run(["open", path])
    else:  # Linux
        subprocess.run(["xdg-open", path])


# ─────────────────────────────────────────────────────────────────
#  WIZARD GUI
# ─────────────────────────────────────────────────────────────────

class ANTWizard:
    def __init__(self, root):
        self.root = root
        self.root.title("ANT System Setup - New Storage Wizard v.2.0.0")
        self.root.geometry("750x550")
        self.root.resizable(False, False)
        self.root.configure(bg=BG)
        
        # Center window
        self.root.update_idletasks()
        x = (self.root.winfo_screenwidth() // 2) - (750 // 2)
        y = (self.root.winfo_screenheight() // 2) - (550 // 2)
        self.root.geometry(f"+{x}+{y}")
        
        # Data
        self.current_step = 1
        self.selected_drive = tk.StringVar()
        self.storage_name = tk.StringVar()
        self.sanitized_name = tk.StringVar()
        self.full_path = ""
        
        # Storage name trace for live sanitization
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
        """Clear current content frame."""
        for widget in self.content_frame.winfo_children():
            widget.destroy()
    
    def update_subtitle(self, text):
        """Update step subtitle."""
        self.subtitle_label.config(text=text)
    
    def update_sanitized_name(self, *args):
        """Update sanitized name preview in real-time."""
        raw = self.storage_name.get()
        sanitized = sanitize_name(raw)
        self.sanitized_name.set(sanitized)
    
    # ─────────────────────────────────────────────────────────────
    #  STEP 1: SELECT DRIVE
    # ─────────────────────────────────────────────────────────────
    
    def show_step_1(self):
        self.current_step = 1
        self.update_subtitle("Step 1 of 3: Select Drive")
        self.clear_content()

        self.back_btn.config(state="disabled")
        self.next_btn.config(text="Next →", command=self.go_next)

        if IS_WINDOWS:
            drives = get_available_drives()

            info = tk.Label(
                self.content_frame,
                text="Select the drive where your ANT Storage will be created:",
                font=("Segoe UI", 10),
                bg=BG,
                fg=TEXT,
                justify="left"
            )
            info.pack(anchor="w", pady=(0, 20))

            drive_frame = tk.Frame(self.content_frame, bg=BG)
            drive_frame.pack(fill="x", pady=10)

            tk.Label(
                drive_frame,
                text="Available drives:",
                font=("Segoe UI", 10),
                bg=BG,
                fg=TEXT
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

            # Warning frame (oculto inicialmente)
            warning_frame = tk.Frame(self.content_frame, bg="#3d2a1f", bd=1, relief="solid")
            warning_title = tk.Label(
                warning_frame,
                text="⚠️  WARNING - Data Integrity Risk",
                font=("Segoe UI", 10, "bold"),
                bg="#3d2a1f",
                fg=ACCENT
            )
            warning_title.pack(anchor="w", padx=10, pady=(10, 5))

            warning_text = tk.Label(
                warning_frame,
                text=(
                    "Storing your files on C: is not recommended:\n\n"
                    "• Windows Updates may wipe your data.\n"
                    "• C: is typically slower and more fragmented.\n"
                    "• ANT is designed for be portability on external drives."
                ),
                font=("Segoe UI", 9),
                bg="#3d2a1f",
                fg="#d4d4d4",
                justify="left"
            )
            warning_text.pack(anchor="w", padx=10, pady=(0, 10))

            # Función para mostrar/ocultar la advertencia
            def update_warning(event=None):
                selected = self.selected_drive.get().upper()
                if selected.startswith("C"):
                    warning_frame.pack(fill="x", pady=(30, 0), padx=10)
                else:
                    warning_frame.pack_forget()

            # Vincular evento de selección
            self.drive_combo.bind("<<ComboboxSelected>>", update_warning)

            # Mostrar/ocultar según valor inicial
            update_warning()

            
        else:
            # Unix: Entry + Browse button
            info = tk.Label(
                self.content_frame,
                text="Enter the base path where the storage will be created:",
                font=("Segoe UI", 10),
                bg=BG,
                fg=TEXT,
                justify="left"
            )
            info.pack(anchor="w", pady=(0, 10))
            
            example = tk.Label(
                self.content_frame,
                text="Example: /home/user  or  /Volumes/MyDrive",
                font=("Segoe UI", 9),
                bg=BG,
                fg="#888888",
                justify="left"
            )
            example.pack(anchor="w", pady=(0, 20))
            
            path_frame = tk.Frame(self.content_frame, bg=BG)
            path_frame.pack(fill="x", pady=10)
            
            self.path_entry = tk.Entry(
                path_frame,
                textvariable=self.selected_drive,
                font=("Segoe UI", 10),
                bg=INPUT_BG,
                fg=TEXT,
                insertbackground=TEXT,
                bd=0,
                relief="flat"
            )
            self.path_entry.pack(side="left", fill="x", expand=True, ipady=8, padx=(0, 10))
            
            browse_btn = tk.Button(
                path_frame,
                text="Browse...",
                font=("Segoe UI", 9),
                bg=INPUT_BG,
                fg=TEXT,
                activebackground="#3a3a3a",
                activeforeground=TEXT,
                bd=0,
                padx=15,
                pady=8,
                cursor="hand2",
                command=self.browse_directory
            )
            browse_btn.pack(side="right")
            
            # Set default
            self.selected_drive.set(os.path.expanduser("~"))
    
    def browse_directory(self):
        """Open directory picker for Unix systems."""
        path = filedialog.askdirectory(
            title="Select Base Directory",
            initialdir=self.selected_drive.get() or os.path.expanduser("~")
        )
        if path:
            self.selected_drive.set(path)
    
    # ─────────────────────────────────────────────────────────────
    #  STEP 2: NAME STORAGE
    # ─────────────────────────────────────────────────────────────
    
    def show_step_2(self):
        self.current_step = 2
        self.update_subtitle("Step 2 of 3: Name Your Storage")
        self.clear_content()
        
        self.back_btn.config(state="normal")
        self.next_btn.config(text="Next →", command=self.go_next)
        
        info = tk.Label(
            self.content_frame,
            text="Choose a descriptive name for your ANT Storage:",
            font=("Segoe UI", 10),
            bg=BG,
            fg=TEXT,
            justify="left"
        )
        info.pack(anchor="w", pady=(0, 10))
        
        example = tk.Label(
            self.content_frame,
            text="Examples: My_Files   Home_Files   Mike_Storage",
            font=("Segoe UI", 9),
            bg=BG,
            fg="#888888",
            justify="left"
        )
        example.pack(anchor="w", pady=(0, 20))
        
        # Name entry
        name_frame = tk.Frame(self.content_frame, bg=BG)
        name_frame.pack(fill="x", pady=10)
        
        tk.Label(
            name_frame,
            text="Storage name:",
            font=("Segoe UI", 10),
            bg=BG,
            fg=TEXT
        ).pack(anchor="w", pady=(0, 5))
        
        name_entry = tk.Entry(
            name_frame,
            textvariable=self.storage_name,
            font=("Segoe UI", 11),
            bg=INPUT_BG,
            fg=TEXT,
            insertbackground=TEXT,
            bd=0,
            relief="flat"
        )
        name_entry.pack(fill="x", ipady=10)
        name_entry.focus()
        
        # Sanitized preview
        preview_frame = tk.Frame(self.content_frame, bg="#1a2a1f", bd=1, relief="solid")
        preview_frame.pack(fill="x", pady=(20, 0), padx=10)
        
        tk.Label(
            preview_frame,
            text="ℹ️ The wizard will automatically adapt it to a valid name format (Pascal_Snake_Case):",
            font=("Segoe UI", 9),
            bg="#1a2a1f",
            fg="#b0b0b0"
        ).pack(anchor="w", padx=10, pady=(10, 5))
        
        preview_label = tk.Label(
            preview_frame,
            textvariable=self.sanitized_name,
            font=("Consolas", 10, "bold"),
            bg="#1a2a1f",
            fg=BUTTON
        )
        preview_label.pack(anchor="w", padx=10, pady=(0, 10))
    
    # ─────────────────────────────────────────────────────────────
    #  STEP 3: REVIEW & CONFIRM
    # ─────────────────────────────────────────────────────────────
    
    def show_step_3(self):
        self.current_step = 3
        self.update_subtitle("Step 3 of 3: Review & Confirm")
        self.clear_content()
        
        self.back_btn.config(state="normal")
        self.next_btn.config(text="Create", command=self.create_storage)
        
        # Calculate full path
        drive = self.selected_drive.get()
        name = self.sanitized_name.get()
        
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
            bg=INPUT_BG,
            fg=BUTTON,
            anchor="w"
        ).pack(fill="x", padx=10, pady=(2, 8))
        
        # Tree preview
        tk.Label(
            self.content_frame,
            text="Folder structure that will be created:",
            font=("Segoe UI", 10, "bold"),
            bg=BG,
            fg=TEXT
        ).pack(anchor="w", pady=(0, 10))
        
        # Treeview with scrollbar
        tree_frame = tk.Frame(self.content_frame, bg=BG)
        tree_frame.pack(fill="both", expand=True)
        
        scrollbar = tk.Scrollbar(tree_frame, bg=INPUT_BG)
        scrollbar.pack(side="right", fill="y")
        
        self.tree = ttk.Treeview(
            tree_frame,
            selectmode="none",
            yscrollcommand=scrollbar.set,
            show="tree"
        )
        self.tree.pack(side="left", fill="both", expand=True)
        scrollbar.config(command=self.tree.yview)
        
        # Populate tree
        root_node = self.tree.insert("", "end", text=f"📁 {name}/", open=True)
        folder_nodes = {}
        
        for folder in LEVEL1:
            folder_node = self.tree.insert(root_node, "end", text=f"📁 {folder}", open=False)
            folder_nodes[folder] = folder_node
            
            if folder in REVIEW_TARGETS:
                review_name = f"98.{folder[4:]}"
                self.tree.insert(folder_node, "end", text=f"📁 {review_name}")
        
        # Shortcuts note
        review_root_node = folder_nodes.get("998.Review")
        for folder in REVIEW_TARGETS:
            self.tree.insert(review_root_node, "end", text=f"⭧ {folder} (Shortcut to '98.{folder[4:]}')")
    
    # ─────────────────────────────────────────────────────────────
    #  NAVIGATION
    # ─────────────────────────────────────────────────────────────
    
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
            # Validate storage name
            name = self.sanitized_name.get()
            
            if not name:
                messagebox.showerror("Error", "Storage name cannot be empty.")
                return
            
            self.show_step_3()
    
    # ─────────────────────────────────────────────────────────────
    #  CREATE STORAGE
    # ─────────────────────────────────────────────────────────────
    
    def create_storage(self):
        """Create the ANT storage structure."""
        
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
            bg=BG,
            fg=TEXT
        ).pack(pady=(20, 10))
        
        # Log text widget
        log_frame = tk.Frame(progress_win, bg=BG)
        log_frame.pack(fill="both", expand=True, padx=20, pady=(0, 20))
        
        scrollbar = tk.Scrollbar(log_frame)
        scrollbar.pack(side="right", fill="y")
        
        log_text = tk.Text(
            log_frame,
            font=("Consolas", 9),
            bg=TREE_BG,
            fg=TEXT,
            insertbackground=TEXT,
            yscrollcommand=scrollbar.set,
            wrap="word",
            state="disabled"
        )
        log_text.pack(side="left", fill="both", expand=True)
        scrollbar.config(command=log_text.yview)
        
        def log(msg, color=TEXT):
            log_text.config(state="normal")
            log_text.insert("end", msg + "\n", ("log",))
            log_text.tag_config("log", foreground=color)
            log_text.see("end")
            log_text.config(state="disabled")
            progress_win.update()
        
        # Create structure
        try:
            # Root
            os.makedirs(self.full_path, exist_ok=True)
            log(f"Wizard online; Generating root directory ...", BUTTON)
            log(f"✔ {self.full_path}", BUTTON)
            
            review_sandbox_paths = {}
            
            # Level-1 folders + sandboxes
            log(f"\nGenerating rest of directories ...", BUTTON)
            for folder in LEVEL1:
                folder_path = os.path.join(self.full_path, folder)
                os.makedirs(folder_path, exist_ok=True)
                log(f"✔ {self.full_path}\\{folder}", BUTTON)
                
                if folder in REVIEW_TARGETS:
                    review_name = f"98.{folder[4:]}"
                    review_path = os.path.join(folder_path, review_name)
                    os.makedirs(review_path, exist_ok=True)
                    log(f"✔ {self.full_path}\\{folder}\\{review_name}", BUTTON)
                    review_sandbox_paths[folder] = review_path
            
            # Shortcuts
            log("\nGenerating shortcuts ...", ACCENT)
            review_index = os.path.join(self.full_path, "998.Review")
            
            for folder, sandbox_path in review_sandbox_paths.items():
                if IS_WINDOWS:
                    shortcut_path = os.path.join(review_index, f"{folder}.lnk")
                    create_shortcut_windows(sandbox_path, shortcut_path)
                else:
                    link_path = os.path.join(review_index, folder)
                    create_symlink_unix(sandbox_path, link_path)
                log(f"✔ {self.full_path}\\998.Review\\[⤤]{folder} ", BUTTON)
            
            # Download README
            log("\nDownloading elemental files ...", ACCENT)
            readme_dest = os.path.join(self.full_path, README_NAME)
            try:
                urllib.request.urlretrieve(README_URL, readme_dest)
                log(f"✔ {self.full_path}\\{README_NAME} (downloaded)", BUTTON)
            except Exception as e:
                placeholder = (
                    f"# ANT System — Get Started\n\n"
                    f"Could not download the guide automatically.\n"
                    f"Please visit:\n\n"
                    f"  {DOCS_URL}\n\n"
                    f"Error: {e}\n"
                )
                with open(readme_dest, "w", encoding="utf-8") as f:
                    f.write(placeholder)
                log(f"⚠ Download failed. Placeholder created.", ACCENT)
            
            log("\n" + "═" * 50, TEXT)
            log("🐜 Congrats, ANT Storage successfully created!", BUTTON)
            log("═" * 50 + "\n", TEXT)
            
            if IS_WINDOWS:
                os.startfile(readme_dest)
            else:
                subprocess.run(["open" if platform.system() == "Darwin" else "xdg-open", readme_dest])

            # Success buttons
            btn_frame = tk.Frame(progress_win, bg=BG)
            btn_frame.pack(pady=(0, 20))
            
            open_btn = tk.Button(
                btn_frame,
                text="Open Folder",
                font=("Segoe UI", 10, "bold"),
                bg=BUTTON,
                fg=TEXT,
                activebackground="#00a077",
                activeforeground=TEXT,
                bd=0,
                padx=20,
                pady=8,
                cursor="hand2",
                command=lambda: open_folder(self.full_path)
            )
            open_btn.pack(side="left", padx=(0, 10))
            
            close_btn = tk.Button(
                btn_frame,
                text="Close",
                font=("Segoe UI", 10),
                bg=INPUT_BG,
                fg=TEXT,
                activebackground="#3a3a3a",
                activeforeground=TEXT,
                bd=0,
                padx=20,
                pady=8,
                cursor="hand2",
                command=lambda: [progress_win.destroy(), self.root.destroy()]
            )
            close_btn.pack(side="left")
            
        except Exception as e:
            log(f"\n❌ ERROR: {str(e)}", "#ff4444")
            messagebox.showerror("Error", f"Failed to create storage:\n{str(e)}")
            progress_win.destroy()


# ─────────────────────────────────────────────────────────────────
#  ENTRY POINT
# ─────────────────────────────────────────────────────────────────

def main():
    root = tk.Tk()
    app = ANTWizard(root)
    root.mainloop()


if __name__ == "__main__":
    main()