# Builds the storage on disk: folders from Tree.yaml, review shortcuts and downloaded core files.
import os
import re
import json
import shutil
import tempfile
import string
import urllib.request

import yaml

from ant_core.theme import TEXT, ACCENT, ERROR
from ant_core.tree_types import get_type_base, child_entries
from ant_core.shortcuts import IS_WINDOWS, review_shortcut_name, create_shortcut

from .config import CORE_API_URL, TREE_YAML_URL


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


def rollback_created(created: list, log_fn) -> list:
    """Remove only the folders this run created, deepest first; keep and return the ones that remain."""
    remaining = []
    for path in reversed(created):
        try:
            shutil.rmtree(path)
        except FileNotFoundError:
            pass  # already removed together with its parent
        except OSError as e:
            remaining.append(path)
            log_fn(f"✖ {path} (Not deleted: {e})", ERROR)
            continue
        log_fn(f"◄◄ {path} (Deleted)", ACCENT)
    created[:] = list(reversed(remaining))
    return remaining


def write_file_atomic(path: str, text: str):
    """Write next to the target and swap it in, so a failure never leaves a half-written file."""
    fd, tmp_path = tempfile.mkstemp(prefix=".ant-", suffix=".tmp", dir=os.path.dirname(path))
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as f:
            f.write(text)
        os.replace(tmp_path, path)
    finally:
        if os.path.exists(tmp_path):
            os.remove(tmp_path)


def download_tree_yaml() -> tuple:
    """Fetch and parse Tree.yaml in memory; nothing touches the disk until it is valid."""
    req = urllib.request.Request(TREE_YAML_URL, headers={"User-Agent": "ANT-Setup-Wizard"})
    with urllib.request.urlopen(req, timeout=15) as r:
        raw = r.read().decode("utf-8")
    parsed = yaml.safe_load(raw)
    if not isinstance(parsed, dict):
        raise ValueError("Tree.yaml is empty or not a mapping")
    return raw, parsed


def process_yaml_tree(tree_dict: dict, base_path: str, review_index: str,
                      log_fn, created: list, path_context: list = None):
    """Create the template folders; every folder made here is added to created for rollback."""
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
                process_yaml_tree(children, folder_path, review_index, log_fn, created, ctx_next)

        elif type_prefix == 'REV':
            make_dirs_tracked(folder_path, created)
            log_fn(f"✔ {folder_path}")

            try:
                shortcut_name = review_shortcut_name(path_context, key)
                create_shortcut(folder_path, os.path.join(review_index, shortcut_name))
                log_fn(f"  ⤤ shortcut created → {shortcut_name}", TEXT)
            except (OSError, ValueError) as e:
                log_fn(f"  ⚠ {e}", ACCENT)

        else:
            # NODE, WKB, RES, PORTF, WS, etc.
            make_dirs_tracked(folder_path, created)
            log_fn(f"✔ {folder_path}")
            if children:
                process_yaml_tree(children, folder_path, review_index, log_fn, created, ctx_next)


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
