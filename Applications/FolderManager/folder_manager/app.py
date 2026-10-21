# Main FolderManager window: tree state, selection events and refresh.
import os
import tkinter as tk

from ant_core.theme import BG, TEXT, ACCENT, BUTTON, DISABLED_TEXT, DELETE_BTN, GHOST_TEXT
from ant_core.tree_types import get_type_base, child_entries

from .config import CURRENT_VERSION, APP_DIR, ICON_OPEN, ICON_CLOSED, ICON_GHOST, TYPE_LABELS, TYPE_DISPLAY, STATUS_OPTIONS, status_from_yaml
from .names import find_root, list_subdirs, get_editable_part
from .yaml_store import sync_yaml, get_yaml_node, get_yaml_type, is_locked_type
from .review_index import rebuild_review_index
from .tree_node import TreeNode
from .ui import UiMixin
from .edit_actions import EditActionsMixin
from .move_delete import MoveDeleteMixin


class FolderManagerApp(UiMixin, EditActionsMixin, MoveDeleteMixin):
    def __init__(self, root: tk.Tk, start_path: str):
        self.tk_root   = root
        self.root_path = find_root(start_path)

        root.title(f"ANT System - Folder Manager {CURRENT_VERSION}")
        root.geometry("750x550")
        root.resizable(True, True)
        root.configure(bg=BG)
        root.iconbitmap(os.path.join(APP_DIR, "FolderManager.ico"))

        self.yaml_data = sync_yaml(self.root_path)

        self.top_nodes: list[TreeNode] = []
        self.visible_nodes: list[TreeNode] = []
        self.selected_node: TreeNode | None = None

        self._build_ui()
        self._init_tree()
        self._render_tree()

    def _init_tree(self):
        self.top_nodes = self._build_nodes(self.root_path, [], 0)

    def _build_nodes(self, disk_path: str | None, rel_parts: list[str], depth: int) -> list[TreeNode]:
        # Dirs on disk
        disk_dirs: list[str] = []
        if disk_path and os.path.isdir(disk_path):
            disk_dirs = list_subdirs(disk_path)

        # Yaml keys at this level
        yaml_node = get_yaml_node(self.yaml_data, rel_parts) or {}
        yaml_keys = list(child_entries(yaml_node))

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

    def _refresh(self):
        """Re-scan disk, update yaml with new dirs (no overwrites), rebuild tree."""
        self.yaml_data  = sync_yaml(self.root_path)
        failures = rebuild_review_index(self.root_path, self.yaml_data)
        expansion_state = self._capture_expansion()
        self.selected_node = None
        self._set_actions_enabled(False)
        self._init_tree()
        self._restore_expansion(expansion_state)
        self._render_tree()
        if failures:
            self._flash(f"{len(failures)} review shortcut(s) failed: {failures[0]}", error=True)

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
