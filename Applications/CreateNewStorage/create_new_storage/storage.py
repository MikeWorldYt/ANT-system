# Builds the storage on disk: folders from Tree.yaml, review shortcuts and downloaded core files.
import os
import re
import json
import shutil
import string
import urllib.request

from ant_core.theme import TEXT, ACCENT
from ant_core.tree_types import get_type_base, child_entries
from ant_core.shortcuts import IS_WINDOWS, review_shortcut_name, create_shortcut

from .config import CORE_API_URL


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


def make_dirs_tracked(path: str, created: list):
    """makedirs that records every folder it actually creates, parents first."""
    missing = []
    current = os.path.abspath(path)
    while not os.path.exists(current):
        missing.append(current)
        parent = os.path.dirname(current)
        if parent == current:
            break
        current = parent
    os.makedirs(path, exist_ok=True)
    created.extend(reversed(missing))


def rollback_created(created: list):
    """Remove only the folders this run created, deepest first."""
    for path in reversed(created):
        shutil.rmtree(path, ignore_errors=True)
    created.clear()


def process_yaml_tree(tree_dict: dict, base_path: str, review_index: str,
                      log_fn, path_context: list = None):
    if path_context is None:
        path_context = []

    for key, value in child_entries(tree_dict).items():
        type_prefix = get_type_base(value.get('type', ''))
        folder_path = os.path.join(base_path, key)
        ctx_next    = path_context + [key]
        children    = child_entries(value)

        if type_prefix == 'CFG':
            # Already created manually — recurse only
            if children:
                process_yaml_tree(children, folder_path, review_index, log_fn, ctx_next)

        elif type_prefix == 'REV':
            os.makedirs(folder_path, exist_ok=True)
            log_fn(f"✔ {folder_path}")

            try:
                shortcut_name = review_shortcut_name(path_context)
                create_shortcut(folder_path, os.path.join(review_index, shortcut_name))
                log_fn(f"  ⤤ shortcut created → {shortcut_name}", TEXT)
            except (OSError, ValueError) as e:
                log_fn(f"  ⚠ {e}", ACCENT)

        else:
            # NODE, WKB, RES, PORTF, WS, etc.
            os.makedirs(folder_path, exist_ok=True)
            log_fn(f"✔ {folder_path}")
            if children:
                process_yaml_tree(children, folder_path, review_index, log_fn, ctx_next)


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
