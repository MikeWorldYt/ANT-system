# Worknodes

## Names and numbers
- A worknode (WKB) is a project folder: a tracker, a number and a title, `PJ.001_Web`. Trackers: `PJ.` project, `PR.` practice, `HW.` homework, `CR.` course, `TK.` task.
- Each folder counts its own worknodes per tracker: the next number follows the highest among the siblings, at the width they use (`PJ.051` then `PJ.052`). `add_worknode` numbers it; never type one.
- A worknode moved into another folder continues that folder's count.
- `set_type` makes a folder a NODE or a WKB.

## Details
- `set_status`: `in-progress` (the default when unset), `for-review`, `hold`, `done`.
- `set_owner`: who the work is for, up to 100 characters.
- `set_output`: the folder or file inside the worknode that is its deliverable, e.g. `render`; null clears it. It must exist inside the worknode.
- `set_favorite` stars it in the sidebar, `set_important` marks it, `set_board` places it in the task board's columns and order, `hide` takes it out of lists and search.

## Outcomes
- `00.<Title>` (type OUT) next to worknodes is the level's Outcomes folder; its title can change, its id cannot.
- Delivering a worknode in the app moves its output into `(date)_<Title>` inside Outcomes and leaves a shortcut behind. The MCP sets the output; delivering is done in the app.

## Plain folders inside a worknode
- A worknode keeps working folders as the user names them, without an id and untracked by the tree: `assets/kit`, `render`. Make them with `add_folder`; `add_node` would give an id.
- Files move into them with `move_files` (see ant://guide/files).
