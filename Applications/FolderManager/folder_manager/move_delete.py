# Reorder nodes by swapping their numeric ids, and delete folders or ghost records.
import os
import shutil
from tkinter import messagebox

from .names import get_numeric_id, get_editable_part, is_movable_node, list_subdirs
from .yaml_store import save_yaml, get_yaml_node, get_yaml_type, delete_yaml_key


class MoveDeleteMixin:
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
            return
        # Special rule: id 01 cannot move to 00
        if direction == -1 and node_id == 1:
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

        save_yaml(self.root_path, self.yaml_data)

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
        expansion_state = self._capture_expansion()
        self._init_tree()
        self._restore_expansion(expansion_state)
        self._render_tree()
