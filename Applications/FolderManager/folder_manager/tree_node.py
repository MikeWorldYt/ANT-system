# One row of the folder tree, lazily expanded; ghosts exist in Tree.yaml but not on disk.
from .names import list_subdirs


class TreeNode:
    def __init__(self, abs_path: str | None, depth: int, rel_parts: list[str], ghost: bool = False):
        self.abs_path  = abs_path
        self.depth     = depth
        self.rel_parts = rel_parts
        self.name      = rel_parts[-1] if rel_parts else ""
        self.ghost     = ghost          # True = exists in yaml but not on disk
        self.expanded  = False
        self.children: list["TreeNode"] = []
        self._loaded   = False

    def has_children(self) -> bool:
        if self.ghost:
            return False
        return bool(list_subdirs(self.abs_path))

    def flat_visible(self) -> list["TreeNode"]:
        result = [self]
        if self.expanded:
            for child in self.children:
                result.extend(child.flat_visible())
        return result

    def refresh_children(self):
        self._loaded = False
        self.children = []
