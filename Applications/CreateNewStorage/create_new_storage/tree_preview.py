# Shows the folders Tree.yaml will create in the review step.
from tkinter import ttk

from ant_core.tree_types import get_type_base, child_entries


def build_tree_widget(tree_dict: dict, parent_node, treeview: ttk.Treeview,
                      depth: int = 0):
    ICONS = {
        'CFG':   '⚙',
        'REV':   '♻',
        'RES':   '📦',
        'NODE':  '📁',
        'PORTF': '🗃',
        'WKB':   '📋',
        'WS':    '📂',
    }

    for key, value in child_entries(tree_dict).items():
        icon        = ICONS.get(get_type_base(value.get('type', '')), '📁')
        children    = child_entries(value)
        auto_open   = depth == 0 and bool(children)

        node = treeview.insert(parent_node, "end",
                               text=f"{icon} {key}", open=auto_open)

        # Annotate special folders
        if key == '998.Review':
            treeview.insert(node, "end",
                            text="🔗 shortcuts to REV sandboxes (auto-generated)")
        elif key == '00.Core':
            treeview.insert(node, "end", text="📄 Tree.yaml (downloaded)")
            treeview.insert(node, "end", text="📄 + core files (downloaded)")

        if children:
            build_tree_widget(children, node, treeview, depth + 1)
