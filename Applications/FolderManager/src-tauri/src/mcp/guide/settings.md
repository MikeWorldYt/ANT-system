# Settings, extensions and storages

## Settings
`get_settings` reads them; `set_setting` changes one, with the value as text. The open window picks a change up when it shows the setting again or on its next start.
- On or off (`"true"` / `"false"`): `pinFavorites`, `hideDone` (hide done worknodes), `hideDirectoryIdentifiers` (hide ids in names), `confirmDelete`, `autoUpdate` (check for updates), `installUpdates`, `betaUpdates`, `debugLogging`, `backupAuto`, `watchExternal` (scheduled sync), `autoOpenColumns`, `showFavorites`, `showDownloads`, `showReview`, `batchDropToGroup`.
- `onLaunch`: `continue`, `root`.
- `startView`: `continue`, `tree`, `columns`, `grid`, `list`, `details`, `cards`, `graph`.
- `tabView`: `last`, `default`.
- `theme`: `system`, `light`, `dark`. `accent` and `iconColor`: `green`, `blue`, `purple`, `coral`, `gold`. `textSize`: `small`, `medium`, `large`.
- `backupFrequency`: `day`, `week`, `month`. `backupFolder`: a full path.
- `mcpWrites` is the user's alone and is refused here.

## Extensions
- `list_extensions`: the installed ones and their state. `list_extension_library`: what the online library offers.
- `install_extension`, `set_extension_enabled` and `remove_extension` take the extension's `id`; each asks for the user's OK in the window. Install only what the user named.

## Storages
- `list_storages`: the storages Folder Manager knows, which one this server works on, and whether each is still on disk.
- `switch_storage` makes this server work on another listed storage. The open window keeps its own; each storage has its own writes switch. When the window shows another storage, the result warns: the ui_* tools and the window confirmations are refused until the user opens this one there, so tell the user.
