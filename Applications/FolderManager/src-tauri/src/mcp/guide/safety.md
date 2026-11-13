# Safety

- Every change takes the storage lock, as the app does; a busy storage is retried a few times, then refused.
- The open app is told of each change; a warning says when it will only show after a refresh.
- Deletes go to the Recycle Bin. Pass `confirmNonEmpty` or `expectGhost` only after the user agreed to that exact deletion.
- Never write inside `.ant/` or `.ant-trash` by hand; use the tools.
- Locked types (REV, RES, TPL, CFG, SS, REC) and reserved roots refuse changes; do not work around a refusal.
- `ui_get_state` shows the user's tabs and selection; read it only for the task at hand.
