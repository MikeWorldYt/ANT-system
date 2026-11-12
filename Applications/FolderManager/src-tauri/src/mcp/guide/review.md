# Review folders

- A file dropped on a plain node goes to its `REV.<Title>`; a `data` node, a workboard or an OUT keeps it.
- `rename_file` and `delete_file` act on files inside review folders.
- A review folder may carry automation flags, run on every file that lands in it:
  - `auto-reduce` (Clean up names): strips hashes, counters, sites, embedded extensions and repeated symbols, keeping the real extension.
  - `auto-sort`: moves files into the matching workboard (`preview_project_groups` shows the plan, `apply_project_groups` applies it, `undo_project_sort` reverts the last one).
  - `auto-group`: moves files into the folders named by the dictionary in force (see ant://guide/dictionary).
- Files whose name starts with `~` are temporary copies an app holds open: no automation touches them.
