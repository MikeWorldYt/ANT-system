# Model

- Level 1 nodes sit at the root with a 3-digit id in capitals: `100.WORK`, `500.HOBBIES`.
- Deeper nodes use a 2-digit id and a Train-Case title: `08.Art`, `07.Blue-Archive`. The default depth is 3 levels; a storage may allow up to 6.
- `00.` is the Portfolio id, given automatically next to workboards.
- Types (`get_node` returns them as `NODE-L2`, `REV-L3`…, base plus level):
  - NODE: a plain folder of folders. With the `data` flag it also takes files itself.
  - WKB: a workboard (project), named with `PJ.`, `PR.`, `HW.`, `CR.` or `TK.` plus a number: `PJ.001_Web`.
  - OUT: outputs folder of a workboard.
  - REV, RES, TPL: support folders of a node, `REV.<Title>` (review), `RES.<Title>` (resources), `TPL.<Title>` (templates). Older storages use `98.`, `90.`, `97.`.
  - SS, REC: screenshots and recordings, `SS.<Title>` / `REC.<Title>`; the root holds `ANT.SCREENSHOTS` and `ANT.RECORDINGS`.
  - CFG: app settings.
- REV, RES, TPL, CFG, SS and REC are locked: they cannot be renamed, moved, retyped or deleted by hand; they follow their node.
- `.ant/` folders hold app data (`ant.db`, `info.json`, `dictionary.md`); `.ant-trash` is staging. Never treat either as nodes.
- Flags: `hidden` (not listed or searched), `main`, `data`, `auto-reduce`, `auto-sort`, `auto-group`.
