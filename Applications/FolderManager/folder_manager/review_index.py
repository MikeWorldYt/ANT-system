# Rebuilds the 998.Review shortcuts from Tree.yaml.
import os

from ant_core.tree_types import child_entries
from ant_core.shortcuts import review_shortcut_name, create_shortcut

from .config import MAX_DEPTH
from .names import get_editable_part
from .yaml_store import is_locked_type


def rebuild_review_index(root_path: str, yaml_data: dict) -> list[str]:
    """Recreate the 998.Review shortcuts; return the errors of the ones that failed."""
    review_dir = os.path.join(root_path, "998.Review")
    if not os.path.isdir(review_dir):
        return []

    failures: list[str] = []

    for f in os.listdir(review_dir):
        fpath = os.path.join(review_dir, f)
        if os.path.isfile(fpath) and f.lower().endswith(".lnk"):
            try:
                os.remove(fpath)
            except OSError as e:
                failures.append(f"Could not remove {f}: {e}")

    def walk(node: dict, parts: list[str], abs_dir: str):
        for name, child in sorted(child_entries(node).items()):
            if is_locked_type(child.get("type", "")):
                continue
            child_parts = parts + [name]
            child_abs   = os.path.join(abs_dir, name)
            rev_abs     = os.path.join(child_abs, f"98.{get_editable_part(name)}")
            if os.path.isdir(rev_abs):
                try:
                    create_shortcut(rev_abs, os.path.join(review_dir, review_shortcut_name(child_parts)))
                except OSError as e:
                    failures.append(str(e))
            if len(child_parts) < MAX_DEPTH:
                walk(child, child_parts, child_abs)

    walk(yaml_data, [], root_path)
    return failures
