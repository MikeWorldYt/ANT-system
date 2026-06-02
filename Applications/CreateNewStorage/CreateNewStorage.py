"""
╔══════════════════════════════════════════════════════════════════╗
║              ANT SYSTEM — Create New Storage (GUI)               ║
║          Archive Nesting Technique on the System                 ║
║                                                                  ║
║  GUI wizard to initialize a new ANT root storage with full       ║
║  folder structure, Review sandboxes, and shortcuts.              ║
║                                                                  ║
║  Docs:   https://ant-system.vercel.app                          ║
║  GitHub: https://github.com/MikeWorldYt/ANT-system              ║
╚══════════════════════════════════════════════════════════════════╝
"""

import os
import sys
import platform
import string
import re, shutil
import json
import threading
import tempfile
import urllib.request
import tkinter as tk
from tkinter import ttk, messagebox, filedialog
import subprocess

# Auto-install PyYAML if missing
try:
    import yaml
except ImportError:
    import subprocess as _sp
    _sp.check_call([sys.executable, "-m", "pip", "install", "pyyaml", "--quiet"],
                   stdout=_sp.DEVNULL, stderr=_sp.DEVNULL)
    import yaml

# ─────────────────────────────────────────────────────────────────
#  CONSTANTS
# ─────────────────────────────────────────────────────────────────

DOCS_URL       = "https://ant-system.vercel.app/en/start-here/migrating-plan"
README_URL     = "https://raw.githubusercontent.com/MikeWorldYt/ANT-system/main/Migration/01.Get_Started.md"
README_NAME    = "01.Get_Started.md"
TREE_YAML_URL  = "https://raw.githubusercontent.com/MikeWorldYt/ANT-system/main/Migration/Core/Tree.yaml"
TREE_YAML_NAME = "Tree.yaml"
CORE_API_URL        = "https://api.github.com/repos/MikeWorldYt/ANT-system/contents/Migration/Core"
FOLDER_MANAGER_URL  = "https://github.com/MikeWorldYt/ANT-system/raw/main/Applications/FolderManager/dist/FolderManager.exe"
FOLDER_MANAGER_NAME = "FolderManager.exe"
ANT_LOGO_URL        = "https://raw.githubusercontent.com/MikeWorldYt/ANT-system/main/assets/ant.png"
ANT_VERSION         = "v2.3.1"

# ─────────────────────────────────────────────────────────────────
#  COLOR THEME
# ─────────────────────────────────────────────────────────────────

BG       = "#202020"
TEXT     = "#FFFFFF"
BUTTON   = "#00b386"
ACCENT   = "#ff8b40"
INPUT_BG = "#2b2b2b"
TREE_BG  = "#1a1a1a"

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
    subprocess.run(
        ["powershell", "-NoProfile", "-NonInteractive", "-Command", ps_cmd],
        creationflags=subprocess.CREATE_NO_WINDOW,
        capture_output=True
    )


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


def compute_shortcut_name(path_context: list, level: int) -> str:
    if level == 2 and path_context:
        return path_context[0]
    if level == 3 and len(path_context) >= 2:
        return path_context[0][0] + path_context[1]
    return "unknown-shortcut"


def process_yaml_tree(tree_dict: dict, base_path: str, review_index: str,
                      log_fn, path_context: list = None):
    if path_context is None:
        path_context = []

    for key, value in tree_dict.items():
        if key in ('type', 'status') or not isinstance(value, dict):
            continue

        folder_type = value.get('type', '')
        type_prefix = folder_type.split('-')[0] if '-' in folder_type else folder_type
        level       = int(folder_type.split('-L')[-1]) if '-L' in folder_type else 1
        folder_path = os.path.join(base_path, key)
        ctx_next    = path_context + [key]
        children    = {k: v for k, v in value.items()
                       if k not in ('type', 'status') and isinstance(v, dict)}

        if type_prefix == 'CFG':
            # Already created manually — recurse only
            if children:
                process_yaml_tree(children, folder_path, review_index, log_fn, ctx_next)

        elif type_prefix == 'REV':
            os.makedirs(folder_path, exist_ok=True)
            log_fn(f"✔ {folder_path}")

            shortcut_name = compute_shortcut_name(path_context, level)
            if IS_WINDOWS:
                create_shortcut_windows(
                    folder_path, os.path.join(review_index, f"{shortcut_name}.lnk"))
            else:
                create_symlink_unix(folder_path, os.path.join(review_index, shortcut_name))
            log_fn(f"  ⤤ shortcut created → {shortcut_name}", TEXT)

        else:
            # NODE, WKB, RES, PORTF, WS, etc.
            os.makedirs(folder_path, exist_ok=True)
            log_fn(f"✔ {folder_path}")
            if children:
                process_yaml_tree(children, folder_path, review_index, log_fn, ctx_next)


def build_tree_widget(tree_dict: dict, parent_node, treeview: ttk.Treeview,
                      depth: int = 0):
    ICONS = {
        'CFG':   '⚙',
        'REV':   '♻',
        'RES':   '📦',
        'NODE':  '📁',
        'PORTF': '🗃',
        'WKB':   '📋',
        'WS':    '📂',
    }

    for key, value in tree_dict.items():
        if key in ('type', 'status') or not isinstance(value, dict):
            continue

        folder_type = value.get('type', '')
        type_prefix = folder_type.split('-')[0] if '-' in folder_type else folder_type
        icon        = ICONS.get(type_prefix, '📁')
        children    = {k: v for k, v in value.items()
                       if k not in ('type', 'status') and isinstance(v, dict)}
        auto_open   = depth == 0 and bool(children)

        node = treeview.insert(parent_node, "end",
                               text=f"{icon} {key}", open=auto_open)

        # Annotate special folders
        if key == '998.Review':
            treeview.insert(node, "end",
                            text="🔗 shortcuts to REV sandboxes (auto-generated)")
        elif key == '00.Core':
            treeview.insert(node, "end", text="📄 Tree.yaml (downloaded)")
            treeview.insert(node, "end", text="📄 + core files (downloaded)")

        if children:
            build_tree_widget(children, node, treeview, depth + 1)


def download_core_files(core_dir: str, log_fn):
    req = urllib.request.Request(CORE_API_URL,
                                 headers={"User-Agent": "ANT-Setup-Wizard"})
    with urllib.request.urlopen(req, timeout=15) as r:
        items = json.loads(r.read().decode('utf-8'))

    for item in items:
        if item.get('type') != 'file':
            continue
        filename = item['name']
        dest     = os.path.join(core_dir, filename)
        urllib.request.urlretrieve(item['download_url'], dest)
        log_fn(f"✔ {dest} (downloaded) ")


# ─────────────────────────────────────────────────────────────────
#  WIZARD GUI
# ─────────────────────────────────────────────────────────────────

class ANTWizard:
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
        self.logo_path            = ""
        self.logo_img             = None   # keep reference to avoid GC
        
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

            # Función para mostrar/ocultar la advertencia
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
    
    # ─────────────────────────────────────────────────────────────
    #  STEP 2: NAME STORAGE
    # ─────────────────────────────────────────────────────────────
    
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
    
    # ─────────────────────────────────────────────────────────────
    #  STEP 3: REVIEW & CONFIRM
    # ─────────────────────────────────────────────────────────────
    
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
            if not self.sanitized_name.get():
                messagebox.showerror("Error", "Storage name cannot be empty.")
                return
            self.show_step_3()
    
    # ─────────────────────────────────────────────────────────────
    #  CREATE STORAGE
    # ─────────────────────────────────────────────────────────────
    
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
        log_text.tag_config("err",    foreground="#ff4444")
        log_text.tag_config("plain",  foreground=TEXT)
        _TAG = {BUTTON: "ok", ACCENT: "accent", "#ff4444": "err", TEXT: "plain"}
        
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

            # ── Phase 1: Root ─────────────────────────────────────
            os.makedirs(root_path, exist_ok=True)
            log(f"Wizard online\n\nGenerating root directory ...", ACCENT)
            log(f"✔ {root_path}")
            
            # ── Phase 2: Base CFG folders ─────────────────────────
            log(f"\nGenerating ANT Essentials ...", ACCENT)
            os.makedirs(review_index, exist_ok=True)
            log(f"✔ {review_index}")
            os.makedirs(core_dir, exist_ok=True)
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
                    log(f"✖ Error downloading Core YAML: {e}", "#ff4444")
                    shutil.rmtree(self.full_path, ignore_errors=True)
                    log(f"\nApplying roll-back...\n◄◄ {self.full_path} (Deleted)", ACCENT)
                    messagebox.showerror(
                        "Error",
                        f"Failed to download essential file 'Tree.yaml':\n{e}\n\n"
                        "This file is essential to the creation of an ANT Storage.\n"
                        "Please check your internet connection and try again."
                    )
                    # progress_win.destroy()
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
            log(f"\n❌ ERROR: {str(e)}", "#ff4444")
            messagebox.showerror("Error", f"Failed to create storage:\n{str(e)}")
            # progress_win.destroy()


    # ─────────────────────────────────────────────────────────────
    #  COMPLETION SCREEN
    # ─────────────────────────────────────────────────────────────

    def _show_completion(self, progress_win):

        # progress_win.destroy()

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

        if self.logo_path:
            try:
                img = tk.PhotoImage(file=self.logo_path)
                # Scale down to fit ~120px
                factor = max(1, max(img.width(), img.height()) // 120)
                if factor > 1:
                    img = img.subsample(factor, factor)
                self.logo_img = img
                tk.Label(logo_col, image=self.logo_img, bg=BG).pack(expand=True)
            except Exception:
                tk.Label(logo_col, text="🐜", font=("Segoe UI", 52), bg=BG, fg=BUTTON
                         ).pack(expand=True)
        else:
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


# ─────────────────────────────────────────────────────────────────
#  ENTRY POINT
# ─────────────────────────────────────────────────────────────────

def main():
    root = tk.Tk()
    app = ANTWizard(root)
    root.mainloop()


if __name__ == "__main__":
    main()