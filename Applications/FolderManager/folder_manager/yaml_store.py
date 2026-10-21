# Tree.yaml access: load, save, sync with the folders on disk and edit entries by path.
import os

import yaml

from ant_core.tree_types import get_type_base

from .config import (
    SETTINGS_FOLDER, YAML_FILENAME, MAX_DEPTH, LOCKED_TYPES, WKB_IDENTIFIERS, SPECIAL_TYPES,
)


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


def is_locked_type(type_str: str) -> bool:
    return get_type_base(type_str) in LOCKED_TYPES


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
