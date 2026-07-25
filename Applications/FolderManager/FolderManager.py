import tkinter as tk
from tkinter import ttk, messagebox
import os, sys, re, shutil
import yaml

# ─────────────────────────────────────────────────────────────────
#  VERSION
# ─────────────────────────────────────────────────────────────────

CURRENT_VERSION   = "v.1.2.1"
GITHUB_RELEASE_API = "https://api.github.com/repos/MikeWorldYt/ANT-system/releases/latest"
EXE_DOWNLOAD_URL  = "https://github.com/MikeWorldYt/ANT-system/raw/main/Applications/FolderManager/dist/FolderManager.exe"
UPDATER_SCRIPT    = "FM_Updater.bat"

# ─────────────────────────────────────────────────────────────────
#  COLOR THEME
# ─────────────────────────────────────────────────────────────────

BG            = "#202020"
TEXT          = "#FFFFFF"
TITLE         = "#00b386"
BUTTON        = "#00b386"
ACCENT        = "#ff8b40"
INPUT_BG      = "#2b2b2b"
TREE_BG       = "#1a1a1a"
HIGHLIGHT     = "#2a4a3e"
DISABLED_TEXT = "#555555"
BORDER        = "#333333"
HOVER_BTN     = "#00c99a"
DELETE_BTN    = "#c0392b"
DELETE_HOVER  = "#e74c3c"
GHOST_TEXT    = "#8b2020"

ICON_OPEN     = "📂"
ICON_CLOSED   = "📁"
ICON_GHOST    = "👻"

# ─────────────────────────────────────────────────────────────────
#  CONSTANTS
# ─────────────────────────────────────────────────────────────────

SETTINGS_FOLDER = "999.Settings"
YAML_FILENAME   = "Tree.yaml"
MAX_DEPTH       = 3

LOCKED_TYPES    = {"REV", "RES", "TPL", "CFG"}
LINKED_PREFIXES = ("90", "98")
WKB_IDENTIFIERS = ("PJ.", "PR.", "HW.")

SPECIAL_TYPES = {
    "90.": "RES",
    "97.": "TPL",
    "98.": "REV",
}

# Display labels for the Type dropdown (internal_key → label)
TYPE_LABELS: dict[str, str] = {
    "NODE":  "Node",
    "WKB":   "Workboard",
    "PORTF": "Portfolio",
}
TYPE_KEYS   = list(TYPE_LABELS.keys())          # ["NODE", "WKB", "PORTF"]
TYPE_DISPLAY = list(TYPE_LABELS.values())        # ["Node", "Workboard", "Portfolio"]

# Status options for WKB nodes
STATUS_OPTIONS = ["In progress", "For review", "Hold", "Done"]
# Stored in yaml as lowercase-hyphenated
def status_to_yaml(s: str) -> str:
    return s.lower().replace(" ", "-")
def status_from_yaml(s: str) -> str:
    return s.replace("-", " ").title()

# ─────────────────────────────────────────────────────────────────
#  YAML HELPERS
# ─────────────────────────────────────────────────────────────────

def yaml_path(root_path: str) -> str:
    return os.path.join(root_path, SETTINGS_FOLDER, "00.Core", YAML_FILENAME)

def load_yaml(root_path: str) -> dict:
    path = yaml_path(root_path)
    if not os.path.isfile(path):
        return {}
    with open(path, "r", encoding="utf-8") as f:
        return yaml.safe_load(f) or {}

def save_yaml(root_path: str, data: dict):
    path = yaml_path(root_path)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        yaml.dump(data, f, default_flow_style=False, allow_unicode=True, sort_keys=True)

def infer_type(name: str, depth: int) -> str:
    basename = os.path.basename(name)
    if basename.startswith(WKB_IDENTIFIERS):
        return "WKB"
    for prefix, folder_type in SPECIAL_TYPES.items():
        if basename.startswith(prefix):
            return folder_type
    return f"NODE-L{depth + 1}"

def get_type_base(type_str: str) -> str:
    return type_str.split("-")[0] if type_str else ""

def get_type_level(type_str: str) -> str:
    parts = type_str.split("-")
    return parts[1] if len(parts) > 1 else ""

def is_locked_type(type_str: str) -> bool:
    return get_type_base(type_str) in LOCKED_TYPES


# ─────────────────────────────────────────────────────────────────
#  YAML SYNC
# ─────────────────────────────────────────────────────────────────

def collect_disk_dirs(root_path: str) -> set[tuple]:
    """Return set of rel_parts tuples for all dirs up to MAX_DEPTH."""
    result = set()
    def recurse(path: str, parts: tuple, depth: int):
        if depth >= MAX_DEPTH:
            return
        try:
            dirs = sorted(d for d in os.listdir(path) if os.path.isdir(os.path.join(path, d)))
        except PermissionError:
            return
        for d in dirs:
            new_parts = parts + (d,)
            result.add(new_parts)
            recurse(os.path.join(path, d), new_parts, depth + 1)
    recurse(root_path, (), 0)
    return result

def sync_yaml(root_path: str) -> dict:
    """Add new disk dirs to yaml. Does NOT remove missing ones (kept as ghosts)."""
    data = load_yaml(root_path)

    def recurse_sync(disk_path: str, yaml_node: dict, depth: int):
        if depth >= MAX_DEPTH:
            return
        try:
            dirs = sorted(d for d in os.listdir(disk_path)
                          if os.path.isdir(os.path.join(disk_path, d)))
        except PermissionError:
            return
        for d in dirs:
            if d not in yaml_node:
                yaml_node[d] = {"type": infer_type(d, depth)}
            entry = yaml_node[d]
            if not isinstance(entry, dict):
                yaml_node[d] = {"type": infer_type(d, depth)}
                entry = yaml_node[d]
            recurse_sync(os.path.join(disk_path, d), entry, depth + 1)
    recurse_sync(root_path, data, 0)
    save_yaml(root_path, data)
    return data

def get_yaml_node(data: dict, rel_parts: list[str]) -> dict | None:
    """Navigate yaml and return the node dict, or None."""
    node = data
    for part in rel_parts:
        if not isinstance(node, dict) or part not in node:
            return None
        node = node[part]
    return node if isinstance(node, dict) else None

def get_yaml_type(data: dict, rel_parts: list[str]) -> str:
    node = get_yaml_node(data, rel_parts)
    return node.get("type", "") if node else ""

def update_yaml_key(data: dict, parent_parts: list[str], old_key: str, new_key: str):
    node = data
    for part in parent_parts:
        if not isinstance(node, dict) or part not in node:
            return
        node = node[part]
    if isinstance(node, dict) and old_key in node:
        node[new_key] = node.pop(old_key)

def delete_yaml_key(data: dict, parent_parts: list[str], key: str):
    node = data
    for part in parent_parts:
        if not isinstance(node, dict) or part not in node:
            return
        node = node[part]
    if isinstance(node, dict) and key in node:
        del node[key]

def collect_yaml_keys(yaml_node: dict, depth: int = 0) -> list[str]:
    """Return all non-'type'/'status' keys (child folder names)."""
    skip = {"type", "status"}
    result = []
    for k, v in yaml_node.items():
        if k not in skip and isinstance(v, dict):
            result.append(k)
    return result


# ─────────────────────────────────────────────────────────────────
#  REVIEW INDEX (shortcuts inside 998.Review)
# ─────────────────────────────────────────────────────────────────

def _shortcut_name_L1(l1_name: str) -> str:
    """e.g. "100.PERSONAL" """
    return l1_name

def _shortcut_name_L2(l1_name: str, l2_name: str) -> str:
    """e.g. "100.PERSONAL" + "04.Curriculum" → "104.Curriculum" """
    return l1_name[0] + l2_name

def _shortcut_name_L3(l1_name: str, l2_name: str, l3_name: str) -> str:
    """e.g. "400.ACADEMIC" + "01.Math" + "02.Algebra" → "401.02.Algebra" """
    l2_id = l2_name.split(".")[0]          # "01"
    return l1_name[0] + l2_id + "." + l3_name


def _create_shortcut_windows(link_path: str, target_path: str):
    """Create a Windows .lnk shortcut using PowerShell."""
    ps = (
        f'$ws = New-Object -ComObject WScript.Shell; '
        f'$s = $ws.CreateShortcut("{link_path}"); '
        f'$s.TargetPath = "{target_path}"; '
        f'$s.Save()'
    )
    import subprocess
    subprocess.run(
        ["powershell", "-NoProfile", "-NonInteractive", "-Command", ps],
        capture_output=True,
        creationflags=subprocess.CREATE_NO_WINDOW,
    )


def rebuild_review_index(root_path: str, yaml_data: dict):
    review_dir = os.path.join(root_path, "998.Review")
    if not os.path.isdir(review_dir):
        return

    # Remove existing shortcuts
    for f in os.listdir(review_dir):
        fpath = os.path.join(review_dir, f)
        if os.path.isfile(fpath) and f.lower().endswith(".lnk"):
            try:
                os.remove(fpath)
            except OSError:
                pass

    skip_keys = {"type", "status"}

    # Walk yaml up to MAX_DEPTH
    for l1_name, l1_node in sorted(yaml_data.items()):
        if not isinstance(l1_node, dict):
            continue
        l1_type = l1_node.get("type", "")
        if get_type_base(l1_type) in LOCKED_TYPES:
            continue
        l1_abs = os.path.join(root_path, l1_name)

        # L1 REV → shortcut named l1_name targeting l1/98.editable
        l1_editable = get_editable_part(l1_name)
        rev_l1_name = f"98.{l1_editable}"
        rev_l1_abs  = os.path.join(l1_abs, rev_l1_name)
        if os.path.isdir(rev_l1_abs):
            link_name = _shortcut_name_L1(l1_name) + ".lnk"
            _create_shortcut_windows(
                os.path.join(review_dir, link_name), rev_l1_abs)

        # L2 children
        for l2_name, l2_node in sorted(l1_node.items()):
            if l2_name in skip_keys or not isinstance(l2_node, dict):
                continue
            l2_type = l2_node.get("type", "")
            if get_type_base(l2_type) in LOCKED_TYPES:
                continue
            l2_abs = os.path.join(l1_abs, l2_name)

            # L2 REV → shortcut named L1[0]+l2_name targeting l2/98.editable
            l2_editable = get_editable_part(l2_name)
            rev_l2_name = f"98.{l2_editable}"
            rev_l2_abs  = os.path.join(l2_abs, rev_l2_name)
            if os.path.isdir(rev_l2_abs):
                link_name = _shortcut_name_L2(l1_name, l2_name) + ".lnk"
                _create_shortcut_windows(
                    os.path.join(review_dir, link_name), rev_l2_abs)

            # L3 children (they ARE the REV, no 98. inside)
            for l3_name, l3_node in sorted(l2_node.items()):
                if l3_name in skip_keys or not isinstance(l3_node, dict):
                    continue
                l3_type = l3_node.get("type", "")
                if get_type_base(l3_type) in LOCKED_TYPES:
                    continue
                l3_abs = os.path.join(l2_abs, l3_name)

                # L3 REV → shortcut named L1[0]+L2id+"."+l3_name targeting l3 folder
                rev_l3_name = f"98.{get_editable_part(l3_name)}"
                rev_l3_abs  = os.path.join(l3_abs, rev_l3_name)
                if os.path.isdir(rev_l3_abs):
                    link_name = _shortcut_name_L3(l1_name, l2_name, l3_name) + ".lnk"
                    _create_shortcut_windows(
                        os.path.join(review_dir, link_name), rev_l3_abs)

# ─────────────────────────────────────────────────────────────────
#  NUMERIC ID HELPERS
# ─────────────────────────────────────────────────────────────────

def get_numeric_id(name: str) -> int | None:
    m = re.match(r'^(\d+)\.', name)
    return int(m.group(1)) if m else None

def next_available_id(siblings: list[str]) -> int:
    used = set()
    for s in siblings:
        n = get_numeric_id(s)
        if n is not None and 1 <= n <= 89:
            used.add(n)
    for i in range(1, 90):
        if i not in used:
            return i
    return 89  # fallback, all slots full

def is_movable_node(name: str, node_type: str) -> bool:
    if get_type_base(node_type) != "NODE":
        return False
    n = get_numeric_id(name)
    return n is not None and 1 <= n <= 89

# ─────────────────────────────────────────────────────────────────
#  NAME HELPERS
# ─────────────────────────────────────────────────────────────────

def sanitize_train_case(raw: str) -> str:
    s = raw.strip()
    s = re.sub(r'[\\/:*?"<>|]', '', s)
    s = re.sub(r'[-\s]+', '-', s)
    s = s.strip('-')
    if not s:
        return s
    return s[0].upper() + s[1:]

def sanitize_upper(raw: str) -> str:
    s = re.sub(r'[\\/:*?"<>|]', '', raw.strip())
    s = re.sub(r'[-\s]+', '-', s).strip('-')
    return s.upper()

def get_editable_part(name: str) -> str:
    return name.split(".", 1)[1] if "." in name else name

def build_new_name(original: str, new_editable: str) -> str:
    return original.split(".", 1)[0] + "." + new_editable if "." in original else new_editable

# ─────────────────────────────────────────────────────────────────
#  ROOT FINDER
# ─────────────────────────────────────────────────────────────────

def find_root(start_path: str) -> str:
    abs_path = os.path.abspath(start_path)
    if os.path.basename(abs_path) == SETTINGS_FOLDER:
        return os.path.dirname(abs_path)
    for parent in [abs_path, os.path.dirname(abs_path)]:
        if os.path.isdir(os.path.join(parent, SETTINGS_FOLDER)):
            return parent
    return abs_path

def list_subdirs(path: str) -> list[str]:
    try:
        return sorted(d for d in os.listdir(path) if os.path.isdir(os.path.join(path, d)))
    except PermissionError:
        return []



# ─────────────────────────────────────────────────────────────────
#  UPDATE CHECK
# ─────────────────────────────────────────────────────────────────

def fetch_latest_version() -> str | None:
    """Query GitHub releases API and return tag_name, or None on failure."""
    try:
        import urllib.request, json
        req = urllib.request.Request(
            GITHUB_RELEASE_API,
            headers={"User-Agent": "FolderManager-Updater"},
        )
        with urllib.request.urlopen(req, timeout=5) as resp:
            data = json.loads(resp.read().decode())
            return data.get("tag_name") 
    except Exception:
        return None


def load_skipped_version(core_path: str) -> str:
    skip_file = os.path.join(core_path, ".skip_version")
    try:
        with open(skip_file, "r") as f:
            return f.read().strip()
    except Exception:
        return ""


def save_skipped_version(core_path: str, version: str):
    skip_file = os.path.join(core_path, ".skip_version")
    try:
        with open(skip_file, "w") as f:
            f.write(version)
    except Exception:
        pass


def launch_updater(core_path: str, exe_path: str):
    """Launch FM_Updater.bat and exit the app."""
    bat = os.path.join(core_path, UPDATER_SCRIPT)
    if not os.path.isfile(bat):
        messagebox.showerror(
            "Updater not found",
            f"Could not find {UPDATER_SCRIPT} in:\n{core_path}"
        )
        return False
    import subprocess
    subprocess.Popen(
        [bat, exe_path, EXE_DOWNLOAD_URL],
        creationflags=subprocess.CREATE_NO_WINDOW,
        shell=True,
    )
    return True


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
        if latest == CURRENT_VERSION:
            return
        skipped = load_skipped_version(core_path)
        if latest == skipped:
            return
        # Schedule dialog on main thread
        parent.after(0, lambda: UpdateDialog(parent, latest, core_path, exe_path))

    t = threading.Thread(target=_worker, daemon=True)
    t.start()

# ─────────────────────────────────────────────────────────────────
#  TREE NODE
# ─────────────────────────────────────────────────────────────────

class TreeNode:
    def __init__(self, abs_path: str | None, depth: int, rel_parts: list[str], ghost: bool = False):
        self.abs_path  = abs_path
        self.depth     = depth
        self.rel_parts = rel_parts
        self.name      = rel_parts[-1] if rel_parts else ""
        self.ghost     = ghost          # True = exists in yaml but not on disk
        self.expanded  = False
        self.children: list["TreeNode"] = []
        self._loaded   = False

    def has_children(self) -> bool:
        if self.ghost:
            return False
        return bool(list_subdirs(self.abs_path))

    def load_children(self):
        if not self._loaded:
            if self.ghost:
                self._loaded = True
                return
            self.children = [
                TreeNode(os.path.join(self.abs_path, d), self.depth + 1, self.rel_parts + [d])
                for d in list_subdirs(self.abs_path)
            ]
            self._loaded = True

    def toggle(self):
        if not self._loaded:
            self.load_children()
        self.expanded = not self.expanded

    def flat_visible(self) -> list["TreeNode"]:
        result = [self]
        if self.expanded:
            for child in self.children:
                result.extend(child.flat_visible())
        return result

    def refresh_children(self):
        self._loaded = False
        self.children = []

# ─────────────────────────────────────────────────────────────────
#  MAIN APPLICATION
# ─────────────────────────────────────────────────────────────────

class FolderManagerApp:
    def __init__(self, root: tk.Tk, start_path: str):
        self.tk_root   = root
        self.root_path = find_root(start_path)

        root.title(f"ANT System - Folder Manager {CURRENT_VERSION}")
        root.geometry("750x550")
        root.resizable(True, True)
        root.configure(bg=BG)
        root.iconbitmap(icon_path)

        try:
            import ctypes
            ctypes.windll.user32.ShowWindow(ctypes.windll.kernel32.GetConsoleWindow(), 0)
        except Exception:
            pass

        self.yaml_data = sync_yaml(self.root_path)
        self.disk_dirs = collect_disk_dirs(self.root_path)

        self.top_nodes: list[TreeNode] = []
        self.visible_nodes: list[TreeNode] = []
        self.selected_node: TreeNode | None = None

        self._build_ui()
        self._init_tree()
        self._render_tree()

    # ─────────────────────────────────────────────────────────────
    #  UI BUILD
    # ─────────────────────────────────────────────────────────────

    def _build_ui(self):
        header = tk.Frame(self.tk_root, bg=BG, pady=6)
        header.pack(fill=tk.X, padx=12)
        tk.Label(header, text=f"Storage Root Name : {os.path.basename(self.root_path)}",
                 bg=BG, fg=TITLE, font=("Segoe UI", 11, "bold")).pack(side=tk.LEFT)

        # ── Toolbar icons ──────────────────────────────────────────
        script_dir = os.path.join(os.path.dirname(os.path.abspath(__file__)), "assets")
        def _load_icon(filename):
            try:
                return tk.PhotoImage(file=os.path.join(script_dir, filename))
            except Exception:
                return None

        self._ico_up      = _load_icon("up.png")
        self._ico_down    = _load_icon("down.png")
        self._ico_plus    = _load_icon("plus.png")
        self._ico_refresh = _load_icon("refresh.png")

        btn_cfg = dict(bg=BG, relief=tk.FLAT, bd=0,
                       activebackground=INPUT_BG, cursor="hand2", padx=4)

        tk.Label(header, text="Actions", bg=BG, fg=TITLE,
                   font=("Segoe UI", 11, "bold")).pack(side=tk.RIGHT, padx=(90, 80))

        self.btn_refresh = tk.Button(
            header, image=self._ico_refresh, text="↺" if not self._ico_refresh else "",
            command=self._refresh, **btn_cfg)
        self.btn_refresh.pack(side=tk.RIGHT, padx=(12, 0))

        self.btn_move_up = tk.Button(
            header, image=self._ico_up, text="▲" if not self._ico_up else "",
            command=self._move_up, **btn_cfg)
        self.btn_move_up.pack(side=tk.RIGHT, padx=(2, 0))

        self.btn_move_down = tk.Button(
            header, image=self._ico_down, text="▼" if not self._ico_down else "",
            command=self._move_down, **btn_cfg)
        self.btn_move_down.pack(side=tk.RIGHT, padx=(2, 0))

        self.btn_add = tk.Button(
            header, image=self._ico_plus, text="+" if not self._ico_plus else "",
            command=self._add_node, **btn_cfg)
        self.btn_add.pack(side=tk.RIGHT, padx=(2, 0))

        main = tk.Frame(self.tk_root, bg=BG)
        main.pack(fill=tk.BOTH, expand=True, padx=12, pady=(0, 12))

        tree_frame = tk.Frame(main, bg=TREE_BG)
        tree_frame.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)
        self._build_tree_panel(tree_frame)

        actions_frame = tk.Frame(main, bg=INPUT_BG, width=220)
        actions_frame.pack(side=tk.RIGHT, fill=tk.Y, padx=(10, 0))
        actions_frame.pack_propagate(False)
        self._build_actions_panel(actions_frame)

    def _build_tree_panel(self, parent):
        sb = tk.Scrollbar(parent, orient=tk.VERTICAL, bg=BORDER, troughcolor=TREE_BG)
        sb.pack(side=tk.RIGHT, fill=tk.Y)
        self.listbox = tk.Listbox(
            parent, bg=TREE_BG, fg=TEXT,
            selectbackground=HIGHLIGHT, selectforeground=TEXT,
            font=("Segoe UI", 10), bd=0, highlightthickness=0,
            activestyle="none", yscrollcommand=sb.set, cursor="arrow",
        )
        self.listbox.pack(fill=tk.BOTH, expand=True)
        sb.config(command=self.listbox.yview)
        self.listbox.bind("<<ListboxSelect>>", self._on_select)
        self.listbox.bind("<Double-Button-1>", self._on_double_click)
        self.listbox.bind("<Return>", self._on_double_click)

    def _build_actions_panel(self, parent):
        # tk.Frame(parent, bg=BORDER, height=1).pack(fill=tk.X, padx=12)

        # Name
        name_row = tk.Frame(parent, bg=INPUT_BG)
        name_row.pack(fill=tk.X, padx=12, pady=(18, 0))
        tk.Label(name_row, text="Name:", bg=INPUT_BG, fg=TEXT,
                 font=("Segoe UI", 10), width=7, anchor="w").pack(side=tk.LEFT)
        self.rename_entry = tk.Entry(
            name_row, bg="#3a3a3a", fg=TEXT, insertbackground=TEXT,
            font=("Segoe UI", 10), bd=0, relief=tk.FLAT,
            highlightthickness=1, highlightcolor=BUTTON, highlightbackground=BORDER,
        )
        self.rename_entry.pack(side=tk.LEFT, fill=tk.X, expand=True, ipady=4)

        # Type
        type_row = tk.Frame(parent, bg=INPUT_BG)
        type_row.pack(fill=tk.X, padx=12, pady=(10, 0))
        tk.Label(type_row, text="Type:", bg=INPUT_BG, fg=TEXT,
                 font=("Segoe UI", 10), width=7, anchor="w").pack(side=tk.LEFT)

        self.type_var = tk.StringVar(value=TYPE_DISPLAY[0])
        style = ttk.Style()
        style.theme_use("clam")
        style.configure("Dark.TCombobox",
            fieldbackground="#3a3a3a", background=BORDER, foreground=TEXT,
            selectbackground="#3a3a3a", selectforeground=TEXT,
            arrowcolor=TEXT, bordercolor=BORDER,
            lightcolor=BORDER, darkcolor=BORDER, padding=(4, 2),
        )
        style.map("Dark.TCombobox",
            fieldbackground=[("disabled", INPUT_BG), ("readonly", "#3a3a3a")],
            foreground=[("disabled", DISABLED_TEXT), ("readonly", TEXT)],
            arrowcolor=[("disabled", DISABLED_TEXT), ("readonly", TEXT)],
            bordercolor=[("disabled", BORDER), ("readonly", BORDER)],
        )
        self.type_combo = ttk.Combobox(
            type_row, textvariable=self.type_var,
            values=TYPE_DISPLAY, state="readonly",
            font=("Segoe UI", 10), style="Dark.TCombobox",
        )
        self.type_combo.pack(side=tk.LEFT, fill=tk.X, expand=True, ipady=2)
        self.type_combo.bind("<<ComboboxSelected>>", self._on_type_changed)

        # Status row (hidden by default, shown for WKB)
        self.status_row = tk.Frame(parent, bg=INPUT_BG)
        tk.Label(self.status_row, text="Status:", bg=INPUT_BG, fg=TEXT,
                 font=("Segoe UI", 10), width=7, anchor="w").pack(side=tk.LEFT)
        self.status_var = tk.StringVar(value=STATUS_OPTIONS[0])
        self.status_combo = ttk.Combobox(
            self.status_row, textvariable=self.status_var,
            values=STATUS_OPTIONS, state="readonly",
            font=("Segoe UI", 10), style="Dark.TCombobox",
        )
        self.status_combo.pack(side=tk.LEFT, fill=tk.X, expand=True, ipady=2)

        # Apply button
        self.rename_btn = tk.Button(
            parent, text="Apply", bg=BUTTON, fg=BG,
            font=("Segoe UI", 10, "bold"), bd=0, relief=tk.FLAT,
            activebackground=HOVER_BTN, activeforeground=BG,
            cursor="hand2", pady=8, command=self._apply_rename,
        )
        self.rename_btn.pack(fill=tk.X, padx=12, pady=(14, 0))

        # Delete button
        self.delete_btn = tk.Button(
            parent, text="Delete", bg=DELETE_BTN, fg=TEXT,
            font=("Segoe UI", 10, "bold"), bd=0, relief=tk.FLAT,
            activebackground=DELETE_HOVER, activeforeground=TEXT,
            cursor="hand2", pady=8, command=self._apply_delete,
        )
        self.delete_btn.pack(fill=tk.X, padx=12, pady=(6, 0))

        # Status label
        self.status_label = tk.Label(
            parent, text="", bg=INPUT_BG, fg=DISABLED_TEXT,
            font=("Segoe UI", 8), anchor="w", wraplength=196, justify=tk.LEFT,
        )
        self.status_label.pack(fill=tk.X, padx=12, pady=(8, 0))

        self._set_actions_enabled(False)

    # ─────────────────────────────────────────────────────────────
    #  TREE INIT — merges disk + yaml ghosts
    # ─────────────────────────────────────────────────────────────

    def _init_tree(self):
        self.top_nodes = self._build_nodes(self.root_path, [], 0)

    def _build_nodes(self, disk_path: str | None, rel_parts: list[str], depth: int) -> list[TreeNode]:
        # Dirs on disk
        disk_dirs: list[str] = []
        if disk_path and os.path.isdir(disk_path):
            disk_dirs = list_subdirs(disk_path)

        # Yaml keys at this level
        yaml_node = get_yaml_node(self.yaml_data, rel_parts) or {}
        yaml_keys = [k for k, v in yaml_node.items()
                     if k not in ("type", "status") and isinstance(v, dict)]

        all_names = sorted(set(disk_dirs) | set(yaml_keys))
        nodes = []
        for name in all_names:
            on_disk   = name in disk_dirs
            in_yaml   = name in yaml_keys
            if on_disk and not in_yaml and any(name.startswith(p + ".") for p in ("90", "98")):
                continue
            child_parts = rel_parts + [name]
            abs_p   = os.path.join(disk_path, name) if (disk_path and on_disk) else None
            ghost   = not on_disk
            nodes.append(TreeNode(abs_p, depth, child_parts, ghost=ghost))
        return nodes

    def _render_tree(self, keep_parts: list[str] | None = None,
                     keep_selection: TreeNode | None = None):
        # keep_parts takes priority; keep_selection is a convenience alias
        target_parts = keep_parts or (keep_selection.rel_parts if keep_selection else None)

        self.visible_nodes = []
        for node in self.top_nodes:
            self.visible_nodes.extend(node.flat_visible())
        self.listbox.delete(0, tk.END)
        reselect_idx = None
        for i, node in enumerate(self.visible_nodes):
            indent  = "    " * node.depth
            locked  = self._is_node_locked(node)
            if node.ghost:
                icon    = ICON_GHOST
                display = f"{indent}{icon} {node.name}"
                self.listbox.insert(tk.END, display)
                self.listbox.itemconfig(i, fg=GHOST_TEXT)
            else:
                has_ch  = node.has_children()
                icon    = (ICON_OPEN if node.expanded else ICON_CLOSED) if has_ch else ICON_CLOSED
                display = f"{indent}{icon} {node.name}"
                self.listbox.insert(tk.END, display)
                if locked:
                    self.listbox.itemconfig(i, fg=DISABLED_TEXT)

            if target_parts and node.rel_parts == target_parts:
                reselect_idx = i

        if reselect_idx is not None:
            self.listbox.selection_set(reselect_idx)
            self.listbox.see(reselect_idx)
            # Sync selected_node to the fresh object
            self.selected_node = self.visible_nodes[reselect_idx]

    def _is_node_locked(self, node: TreeNode) -> bool:
        t = get_yaml_type(self.yaml_data, node.rel_parts)
        return is_locked_type(t)

    # ─────────────────────────────────────────────────────────────
    #  EVENTS
    # ─────────────────────────────────────────────────────────────

    def _on_select(self, _event=None):
        sel = self.listbox.curselection()
        if not sel:
            return
        node = self.visible_nodes[sel[0]]
        self.selected_node = node

        node_type = get_yaml_type(self.yaml_data, node.rel_parts)
        locked    = is_locked_type(node_type)

        if locked:
            self._set_actions_enabled(False)
            # Ghost locked nodes still show Delete
            if node.ghost:
                self.delete_btn.config(state=tk.NORMAL, bg=DELETE_BTN,
                                       fg=TEXT, cursor="hand2")
            self.rename_entry.config(state=tk.NORMAL)
            self.rename_entry.delete(0, tk.END)
            self.rename_entry.config(state=tk.DISABLED)
        else:
            self._set_actions_enabled(True)
            editable = get_editable_part(node.name)
            self.rename_entry.delete(0, tk.END)
            self.rename_entry.insert(0, editable)

            base = get_type_base(node_type)
            label = TYPE_LABELS.get(base, TYPE_DISPLAY[0])
            self.type_var.set(label)

            # Status field visibility
            self._update_status_row(node_type, node)
            self.status_label.config(text="", fg=DISABLED_TEXT)

        # Ghosts always show delete enabled
        if node.ghost and not locked:
            self.delete_btn.config(state=tk.NORMAL, bg=DELETE_BTN,
                                   fg=TEXT, cursor="hand2")

    def _on_type_changed(self, _event=None):
        """Show/hide status row when type dropdown changes."""
        selected_label = self.type_var.get()
        base = next((k for k, v in TYPE_LABELS.items() if v == selected_label), "NODE")
        if base == "WKB":
            self._show_status_row()
        else:
            self._hide_status_row()

    def _update_status_row(self, node_type: str, node: TreeNode):
        base = get_type_base(node_type)
        if base == "WKB":
            yaml_node = get_yaml_node(self.yaml_data, node.rel_parts)
            raw_status = (yaml_node or {}).get("status", "")
            display_status = status_from_yaml(raw_status) if raw_status else STATUS_OPTIONS[0]
            if display_status not in STATUS_OPTIONS:
                display_status = STATUS_OPTIONS[0]
            self.status_var.set(display_status)
            self._show_status_row()
        else:
            self._hide_status_row()

    def _show_status_row(self):
        self.status_row.pack(fill=tk.X, padx=12, pady=(10, 0),
                             after=self.type_combo.master)

    def _hide_status_row(self):
        self.status_row.pack_forget()

    def _on_double_click(self, _event=None):
        sel = self.listbox.curselection()
        if not sel:
            return
        node = self.visible_nodes[sel[0]]
        if node.ghost:
            return
        if node.has_children():
            # Lazy-load with merged ghost+disk children
            if not node._loaded:
                node.children = self._build_nodes(node.abs_path, node.rel_parts, node.depth + 1)
                node._loaded = True
                node.expanded = True
            else:
                node.expanded = not node.expanded
            self._render_tree(keep_selection=node)

    # ─────────────────────────────────────────────────────────────
    #  APPLY RENAME
    # ─────────────────────────────────────────────────────────────

    def _apply_rename(self):
        if not self.selected_node:
            return
        node      = self.selected_node
        old_name  = node.name
        raw_input = self.rename_entry.get()

        selected_label = self.type_var.get()
        new_type_base  = next((k for k, v in TYPE_LABELS.items() if v == selected_label), "NODE")

        if node.depth == 0:
            new_editable = sanitize_upper(raw_input)
        else:
            new_editable = sanitize_train_case(raw_input)

        if not new_editable:
            self._flash("Name cannot be empty.", error=True)
            return

        new_name  = build_new_name(old_name, new_editable)
        old_type  = get_yaml_type(self.yaml_data, node.rel_parts)
        old_level = get_type_level(old_type)
        new_type  = f"{new_type_base}-{old_level}" if old_level else new_type_base

        # Status for WKB
        new_status = status_to_yaml(self.status_var.get()) if new_type_base == "WKB" else None

        if new_name == old_name and new_type == old_type:
            # May still want to update status
            if new_status is not None:
                self._write_yaml_field(node.rel_parts, "status", new_status)
                save_yaml(self.root_path, self.yaml_data)
                self._flash("Status updated.", error=False)
            else:
                self._flash("No changes made.", error=False)
            return

        parent_dir = os.path.dirname(node.abs_path)

        if new_name != old_name:
            new_abs = os.path.join(parent_dir, new_name)
            if os.path.exists(new_abs):
                self._flash("A folder with that name already exists.", error=True)
                return
            try:
                os.rename(node.abs_path, new_abs)
            except OSError as e:
                self._flash(f"Error: {e}", error=True)
                return

            node.abs_path = new_abs
            node.name     = new_name
            node.refresh_children()

            parent_parts = node.rel_parts[:-1]
            update_yaml_key(self.yaml_data, parent_parts, old_name, new_name)
            node.rel_parts = parent_parts + [new_name]

            self._cascade_linked(new_abs, old_name, new_name, node)

        # Update type
        self._write_yaml_field(node.rel_parts, "type", new_type)
        # Update status if WKB
        if new_status is not None:
            self._write_yaml_field(node.rel_parts, "status", new_status)
        elif "status" in (get_yaml_node(self.yaml_data, node.rel_parts) or {}):
            # Type changed away from WKB — remove status
            n = get_yaml_node(self.yaml_data, node.rel_parts)
            if n:
                n.pop("status", None)

        save_yaml(self.root_path, self.yaml_data)

        expansion_state = self._capture_expansion()
        self._init_tree()
        self._restore_expansion(expansion_state)
        self._render_tree(keep_selection=node)

    def _write_yaml_field(self, rel_parts: list[str], field: str, value):
        node = get_yaml_node(self.yaml_data, rel_parts)
        if node is not None:
            node[field] = value

    def _cascade_linked(self, inside_dir: str, old_name: str, new_name: str, source_node: TreeNode):
        old_editable = get_editable_part(old_name)
        new_editable = get_editable_part(new_name)
        if old_editable == new_editable:
            return

        new_node_parts = source_node.rel_parts[:-1] + [new_name]

        for num_prefix in LINKED_PREFIXES:
            child_old = f"{num_prefix}.{old_editable}"
            child_new = f"{num_prefix}.{new_editable}"
            child_old_path = os.path.join(inside_dir, child_old)
            child_new_path = os.path.join(inside_dir, child_new)

            if os.path.isdir(child_old_path):
                if not os.path.exists(child_new_path):
                    try:
                        os.rename(child_old_path, child_new_path)
                    except OSError:
                        pass
                update_yaml_key(self.yaml_data, new_node_parts, child_old, child_new)

    # ─────────────────────────────────────────────────────────────
    #  ADD NODE
    # ─────────────────────────────────────────────────────────────

    def _add_node(self):
        """Create a child node inside the selected folder."""
        if not self.selected_node or self.selected_node.ghost:
            return
        parent_node = self.selected_node
        parent_dir  = parent_node.abs_path
        parent_type = get_yaml_type(self.yaml_data, parent_node.rel_parts)

        # Locked parents can't receive children
        if is_locked_type(parent_type):
            return

        # Depth guard: child would be at parent_node.depth + 1 (0-indexed).
        # MAX_DEPTH=3 means L1/L2/L3 are valid (depths 0/1/2). Depth 3+ is blocked.
        child_depth = parent_node.depth + 1
        if child_depth >= MAX_DEPTH:
            return

        # Siblings = current children on disk
        siblings = list_subdirs(parent_dir)
        new_id   = next_available_id(siblings)
        new_name = f"{new_id:02d}.New-Node"
        new_abs  = os.path.join(parent_dir, new_name)

        if os.path.exists(new_abs):
            return

        try:
            os.mkdir(new_abs)
        except OSError:
            return

        # Always create 98.X on disk regardless of depth
        new_editable = get_editable_part(new_name)
        rev_name     = f"98.{new_editable}"
        rev_abs      = os.path.join(new_abs, rev_name)
        try:
            os.mkdir(rev_abs)
        except OSError:
            pass

        # Register in yaml
        new_type    = f"NODE-L{child_depth + 1}"
        rev_type    = f"REV-L{child_depth + 1}"
        parent_yaml = get_yaml_node(self.yaml_data, parent_node.rel_parts)
        if parent_yaml is not None:
            node_entry: dict = {"type": new_type}
            if child_depth < MAX_DEPTH - 1:
                node_entry[rev_name] = {"type": rev_type}
            parent_yaml[new_name] = node_entry
        save_yaml(self.root_path, self.yaml_data)

        # Refresh tree, keep parent expanded and stay on parent selection
        expansion_state = self._capture_expansion()
        expansion_state.add(tuple(parent_node.rel_parts))
        self._init_tree()
        self._restore_expansion(expansion_state)
        self._render_tree(keep_selection=parent_node)

    # ─────────────────────────────────────────────────────────────
    #  MOVE UP / MOVE DOWN
    # ─────────────────────────────────────────────────────────────

    def _move_up(self):
        self._move_node(direction=-1)

    def _move_down(self):
        self._move_node(direction=1)

    def _move_node(self, direction: int):
        """
        Swap numeric ids between selected node and its nearest movable
        sibling in the given direction (-1=up, +1=down).
        Only works for NODE type, ids 01-89.
        """
        if not self.selected_node or self.selected_node.ghost:
            return
        node      = self.selected_node
        node_type = get_yaml_type(self.yaml_data, node.rel_parts)

        if not is_movable_node(node.name, node_type):
            # debug: self._flash("Only Node folders (01-89) can be moved.", error=True)
            return

        node_id    = get_numeric_id(node.name)
        parent_dir = os.path.dirname(node.abs_path)
        siblings   = list_subdirs(parent_dir)

        # Build sorted list of movable NODE siblings (01-89)
        movable = []
        for s in siblings:
            s_type = get_yaml_type(self.yaml_data, node.rel_parts[:-1] + [s])
            if is_movable_node(s, s_type):
                sid = get_numeric_id(s)
                movable.append((sid, s))
        movable.sort()
        # Find position of current node in movable list
        positions = [m[0] for m in movable]
        if node_id not in positions:
            return
        idx = positions.index(node_id)
        target_idx = idx + direction
        if target_idx < 0 or target_idx >= len(movable):
            # debug: self._flash("Cannot move further in that direction.", error=True)
            return
        # Special rule: id 01 cannot move to 00
        if direction == -1 and node_id == 1:
            # debug: self._flash("Cannot move 01 higher.", error=True)
            return

        swap_id, swap_name = movable[target_idx]

        # Get editable parts (after the dot) without numeric prefix.
        node_editable = get_editable_part(node.name)
        swap_editable = get_editable_part(swap_name)
        node_new_name = f"{swap_id:02d}.{node_editable}"  # node gets swap's id
        swap_new_name = f"{node_id:02d}.{swap_editable}"  # swap gets node's id

        node_abs      = os.path.join(parent_dir, node.name)
        swap_abs      = os.path.join(parent_dir, swap_name)
        node_new_abs  = os.path.join(parent_dir, node_new_name)
        swap_new_abs  = os.path.join(parent_dir, swap_new_name)

        # Collision: both would end up with the same name (e.g. 05.Ships ↔ 06.Ships)
        collision = node_new_name == swap_name
        try:
            if collision:
                temp_abs = os.path.join(parent_dir, node_new_name + "_")
                os.rename(node_abs, temp_abs)
                os.rename(swap_abs, swap_new_abs)
                # Leave temp with trailing _ — user can rename later
                node_new_name = node_new_name + "_"
                node_new_abs  = temp_abs
            else:
                os.rename(node_abs, node_new_abs)
                os.rename(swap_abs, swap_new_abs)
        except OSError as e:
            self._flash(f"Move failed: {e}", error=True)
            return

        parent_parts = node.rel_parts[:-1]

        # Update yaml keys
        parent_yaml = get_yaml_node(self.yaml_data, parent_parts)
        if parent_yaml is not None:
            node_dict = parent_yaml.pop(node.name, {})
            swap_dict = parent_yaml.pop(swap_name, {})
            parent_yaml[node_new_name] = node_dict
            parent_yaml[swap_new_name] = swap_dict
            # def _update_linked(node_dict_inner: dict, old_editable: str, new_editable: str):
            #     if old_editable == new_editable:
            #         return
            #     for prefix in ("90", "98"):
            #         old_child = f"{prefix}.{old_editable}"
            #         new_child = f"{prefix}.{new_editable}"
            #         if old_child in node_dict_inner:
            #             node_dict_inner[new_child] = node_dict_inner.pop(old_child)

            # _update_linked(node_dict, node_editable, swap_editable)
            # _update_linked(swap_dict, swap_editable, node_editable)

        save_yaml(self.root_path, self.yaml_data)
        # debug: self._flash(f'Moved: "{swap_new_name}"', error=False)

        # Update node reference — node takes swap_id, so its new name is node_new_name
        node.abs_path  = node_new_abs
        node.name      = node_new_name
        node.rel_parts = parent_parts + [node_new_name]
        node.refresh_children()

        target_parts = node.rel_parts[:]   # capture before _init_tree recreates nodes
        expansion_state = self._capture_expansion()
        self._init_tree()
        self._restore_expansion(expansion_state)
        self._render_tree(keep_parts=target_parts)

    def _find_node_by_parts(self, rel_parts: list[str]) -> TreeNode | None:
        """Walk top_nodes recursively to find a node by rel_parts."""
        def walk(nodes):
            for n in nodes:
                if n.rel_parts == rel_parts:
                    return n
                if n._loaded:
                    found = walk(n.children)
                    if found:
                        return found
            return None
        return walk(self.top_nodes)

    # ─────────────────────────────────────────────────────────────
    #  DELETE
    # ─────────────────────────────────────────────────────────────

    def _apply_delete(self):
        if not self.selected_node:
            return
        node = self.selected_node
        parent_parts = node.rel_parts[:-1]

        if node.ghost:
            # Only remove from yaml
            delete_yaml_key(self.yaml_data, parent_parts, node.name)
            save_yaml(self.root_path, self.yaml_data)
            self._flash("Record removed from yaml.", error=False)
        else:
            has_content = bool(list_subdirs(node.abs_path)) or bool(
                [f for f in os.listdir(node.abs_path)
                 if os.path.isfile(os.path.join(node.abs_path, f))]
            )
            if has_content:
                confirm = messagebox.askyesno(
                    "Confirm Delete",
                    f'"{node.name}" contains files and/or subfolders.\n\n'
                    "All contents will be permanently deleted. This action is irreversible.\n\n"
                    "Are you sure you want to continue?",
                    icon="warning",
                )
                if not confirm:
                    return
            try:
                shutil.rmtree(node.abs_path)
            except OSError as e:
                self._flash(f"Error deleting folder: {e}", error=True)
                return

            delete_yaml_key(self.yaml_data, parent_parts, node.name)
            save_yaml(self.root_path, self.yaml_data)
    
        self.selected_node = None
        self._set_actions_enabled(False)
        self.disk_dirs = collect_disk_dirs(self.root_path)
        expansion_state = self._capture_expansion()
        self._init_tree()
        self._restore_expansion(expansion_state)
        self._render_tree()

    # ─────────────────────────────────────────────────────────────
    #  HELPERS
    # ─────────────────────────────────────────────────────────────

    def _set_actions_enabled(self, enabled: bool):
        state       = tk.NORMAL if enabled else tk.DISABLED
        combo_state = "readonly" if enabled else tk.DISABLED
        self.rename_entry.config(
            state=state,
            bg="#3a3a3a" if enabled else INPUT_BG,
            fg=TEXT if enabled else DISABLED_TEXT,
            disabledbackground=INPUT_BG,
            disabledforeground=DISABLED_TEXT,
        )
        self.type_combo.config(state=combo_state)
        self.status_combo.config(state=combo_state)
        self.rename_btn.config(
            state=state,
            bg=BUTTON if enabled else BORDER,
            fg=BG if enabled else DISABLED_TEXT,
            cursor="hand2" if enabled else "arrow",
        )
        self.delete_btn.config(
            state=state,
            bg=DELETE_BTN if enabled else BORDER,
            fg=TEXT if enabled else DISABLED_TEXT,
            cursor="hand2" if enabled else "arrow",
        )
        if not enabled:
            self._hide_status_row()

    def _refresh(self):
        """Re-scan disk, update yaml with new dirs (no overwrites), rebuild tree."""
        self.yaml_data  = sync_yaml(self.root_path)
        self.disk_dirs  = collect_disk_dirs(self.root_path)
        rebuild_review_index(self.root_path, self.yaml_data)
        expansion_state = self._capture_expansion()
        self.selected_node = None
        self._set_actions_enabled(False)
        self._init_tree()
        self._restore_expansion(expansion_state)
        self._render_tree()

    def _flash(self, message: str, error: bool):
        self.status_label.config(text=message, fg=ACCENT if error else BUTTON)

    def _capture_expansion(self) -> set[tuple]:
        expanded = set()
        def walk(nodes):
            for n in nodes:
                if n.expanded:
                    expanded.add(tuple(n.rel_parts))
                    walk(n.children)
        walk(self.top_nodes)
        return expanded

    def _restore_expansion(self, expanded: set[tuple]):
        def walk(nodes):
            for n in nodes:
                if tuple(n.rel_parts) in expanded:
                    n.children = self._build_nodes(n.abs_path, n.rel_parts, n.depth + 1)
                    n._loaded  = True
                    n.expanded = True
                    walk(n.children)
        walk(self.top_nodes)


# ─────────────────────────────────────────────────────────────────
#  ENTRY POINT
# ─────────────────────────────────────────────────────────────────

if __name__ == "__main__":
    start = sys.argv[1] if len(sys.argv) > 1 else os.getcwd()
    root  = tk.Tk()
    icon_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "FolderManager.ico")
    root.iconbitmap(icon_path)
    app = FolderManagerApp(root, start_path=start)
    exe_path = os.path.abspath(sys.argv[0])
    root.after(1500, lambda: check_for_updates(root, app.root_path, exe_path))
    root.mainloop()