# FolderManager settings: update URLs, Tree.yaml layout rules and the choices shown in the UI.
import os

from ant_core.version import read_app_version


VERSION_URL       = "https://raw.githubusercontent.com/MikeWorldYt/ANT-system/main/Applications/FolderManager/version.txt"
EXE_DOWNLOAD_URL  = "https://github.com/MikeWorldYt/ANT-system/raw/main/Applications/FolderManager/dist/FolderManager.exe"
UPDATER_SCRIPT    = "FM_Updater.bat"
APP_DIR           = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CURRENT_VERSION   = read_app_version(APP_DIR)

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
