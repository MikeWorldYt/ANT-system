# Files

- `move_files` moves files or plain folders into a folder, as a drop in the app: dropped on a plain node they go to its review folder; a `data` node, a worknode or a plain folder inside one keeps them. Worknode folders dropped on a node or worknode are placed there and numbered.
- `copy_files` copies the same way and leaves the sources.
- Sources may be files in the storage or in Downloads, as `["::Downloads", "<name>"]`; `ui_get_state` lists the user's selection that way.
- A worknode already in the storage under the same id is not placed; the result warns about it.
- `create_shortcut` puts a shortcut to a storage folder or file in a folder outside the storage, such as the Desktop, under a free name. Inside the storage it is refused.

## Downloads
- `list_downloads` with `["::Downloads"]` lists the Downloads folder; add names to go below it. Hidden and system files are left out.
- `rename_download` renames one item there; a name already taken is refused.
- `delete_downloads` sends items there to the Recycle Bin.
- To file a download into the storage, use `move_files`; to clean names in Downloads, use the automations on `["::Downloads"]`.
