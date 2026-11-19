# Automations

## What they do
- `reduce`: cleans names with the dictionary's terms: strips hashes, counters, sites, embedded extensions and repeated symbols, keeping the real extension.
- `sort`: moves files into the matching worknode.
- `group`: moves files into the subfolders or nodes the dictionary names (see ant://guide/dictionary).
- `addDate`: puts the file's date in front of its name. `removeDate` takes a date out and wins over `addDate`.
- `removeSymbols`: takes brackets and stray symbols out, keeping a " (N)" copy counter.

## Where they run
- Review (REV), screenshot (SS) and recording (REC) folders always can, and so can the Downloads folder `["::Downloads"]`. Any other folder needs `set_automations` with `on: true` first.
- `set_review_automation` makes one of them run on its own on every file that lands there: `automation` is one of `reduce`, `sort`, `group`, `addDate`, `removeDate`, `removeSymbols`.

## The flow: plan, show, ask, apply
1. If the task needs dictionary entries, write them with `set_dictionary` (it replaces the whole file; read it with `get_dictionary` first).
2. `plan_automations` with the folder and the automations wanted: a dry run, nothing changes. Each row gives the file, its new name, its project and its destination.
3. Show the user the rows that change, old name to new name and where each goes, as a short table.
4. Ask in the chat whether to run it, unless the user said not to ask again.
5. `apply_automations` with the rows as shown; drop or edit the rows the user changed. The window also asks for the user's OK.
- If the plan is wrong, change the dictionary and plan again; never apply rows the user has not seen.
