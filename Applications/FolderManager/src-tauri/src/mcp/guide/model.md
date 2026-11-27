# Model

## Levels and ids
- Level 1 nodes sit at the root with a 3-digit id in capitals: `100.WORK`, `500.HOBBIES`.
- Deeper nodes use a 2-digit id and a Train-Case title: `08.Art`, `07.Blue-Archive`. The default depth is 3 levels; a storage may allow up to 6 (`get_storage` tells the limit). Past it the tree tracks nothing: `get_tree` stops there and `list_children` reads deeper.
- The app gives every id; never type one. `add_node` takes the lowest free id from 01 to 99 among the siblings, so a gap left by a deleted node is filled first.
- `move` gives the folder the destination's next free id; `copy` keeps the id and adds the next free " (n)". `reorder` swaps the id with the sibling above or below.
- Titles are cleaned as a rename does: spaces become `-`, words Train-Case.
- `00.` is the Portfolio (Outcomes) id, made next to worknodes; see ant://guide/worknodes.

## Types
`get_node` returns them as `NODE-L2`, `REV-L3`…, base plus level.
- NODE: a plain folder of folders. With the `data` flag it also takes files itself.
- WKB: a worknode (project); see ant://guide/worknodes.
- OUT: the Outcomes (Portfolio) folder `00.<Title>` next to worknodes.
- REV, RES, TPL: support folders of a node, `REV.<Title>` (review), `RES.<Title>` (resources), `TPL.<Title>` (templates). Older storages use `98.`, `90.`, `97.`.
- SS, REC: screenshots and recordings, `SS.<Title>` / `REC.<Title>`; the root holds `ANT.SCREENSHOTS` and `ANT.RECORDINGS`.
- The root `ANT.TEMPLATES` (type TPL) holds worknode templates, one unnumbered folder each, named `<identifier><Title>` such as `PJ.Video-Short`. An Outcomes folder's `defaultTemplate` picks one: each new worknode of that node takes its identifier and a copy of its contents.
- CFG: app settings.
- REV, RES, TPL, CFG, SS and REC are locked: they cannot be renamed, moved, retyped or deleted by hand; they follow their node.
- `.ant/` folders hold app data (`ant.db`, `info.json`, `dictionary.md`); `.ant-trash` is staging. Never treat either as nodes.

## Flags
- `hidden`: not listed or searched (`hide`).
- `main`: a root folder, always.
- `data`: a NODE that keeps files itself instead of passing them to its review folder.
- `autom`: a folder that runs automations; review, screenshot and recording folders always do (`set_automations`).
- `auto-reduce`, `auto-sort`, `auto-group`, `auto-add-date`, `auto-remove-date`, `auto-remove-symbols`: automations that run on their own (see ant://guide/automations).
