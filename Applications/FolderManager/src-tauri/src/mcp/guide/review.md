# Review folders

- A file dropped on a plain node goes to its `REV.<Title>`; a `data` node, a worknode, a plain folder inside a worknode or an OUT keeps it.
- `rename_file` and `delete_file` act on files inside review folders and other folders that list files.
- A review folder can run automations on every file that lands in it; see ant://guide/automations.
- `preview_project_groups` shows which worknodes a review folder's loose files would be sorted into, `apply_project_groups` applies it and `undo_project_sort` reverts the last one.
- Files whose name starts with `~` are temporary copies an app holds open, and hidden or system files are never touched: no automation moves them.
