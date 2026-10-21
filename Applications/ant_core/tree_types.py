# Reads the node types stored in Tree.yaml ("NODE-L2", "REV-L3", "CFG"...).

# Keys of a Tree.yaml entry that describe the node instead of naming a child folder
META_KEYS = frozenset({"type", "status"})


def get_type_base(type_str: str) -> str:
    return type_str.split("-")[0] if type_str else ""


def get_type_level(type_str: str) -> str:
    parts = type_str.split("-")
    return parts[1] if len(parts) > 1 else ""


def child_entries(node: dict) -> dict:
    return {k: v for k, v in node.items() if k not in META_KEYS and isinstance(v, dict)}
