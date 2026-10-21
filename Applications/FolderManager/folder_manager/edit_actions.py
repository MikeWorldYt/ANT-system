# Rename, retype and add nodes, keeping disk, linked 90./98. folders and Tree.yaml in step.
import os

from ant_core.tree_types import get_type_level

from .config import MAX_DEPTH, LINKED_PREFIXES, TYPE_LABELS, status_to_yaml
from .names import sanitize_upper, sanitize_train_case, build_new_name, get_editable_part, next_available_id, list_subdirs
from .yaml_store import save_yaml, get_yaml_node, get_yaml_type, update_yaml_key, is_locked_type
from .tree_node import TreeNode


class EditActionsMixin:
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
                    except OSError as e:
                        self._flash(f"Could not rename {child_old}: {e}", error=True)
                        continue
                update_yaml_key(self.yaml_data, new_node_parts, child_old, child_new)

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
        except OSError as e:
            self._flash(f"Could not create folder: {e}", error=True)
            return

        # Always create 98.X on disk regardless of depth
        new_editable = get_editable_part(new_name)
        rev_name     = f"98.{new_editable}"
        rev_abs      = os.path.join(new_abs, rev_name)
        try:
            os.mkdir(rev_abs)
        except OSError as e:
            self._flash(f"Could not create {rev_name}: {e}", error=True)

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
