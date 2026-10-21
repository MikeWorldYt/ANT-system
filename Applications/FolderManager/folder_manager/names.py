# Folder name rules: numeric ids, editable parts, sanitizing and locating the storage root.
import os
import re

from ant_core.tree_types import get_type_base

from .config import SETTINGS_FOLDER


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
