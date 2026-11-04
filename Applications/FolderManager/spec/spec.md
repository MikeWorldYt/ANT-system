# Spec

FolderManager is the desktop app that browses and manages an ANT storage: it shows the storage as a tree, columns, grid or board, keeps the review and search indexes, and applies ANT rules when folders are created, renamed, moved or deleted.

Package: `Applications/FolderManager`, a Tauri 2 app with a Rust backend in `src-tauri/` and a vanilla TypeScript + Vite frontend in `src/` (no UI framework). It is developed in the `folder-manager/` subtree repo and mirrored into `dev/fmanager`.

## Contracts

These stay fixed across refactors; changing one is a feature change, not a refactor.

- IPC command names and argument shapes, `Mutation`, `FmError` and serde camelCase.
- The `storage-changed` event.
- On-disk formats: `.ant/ant.db`, `.ant/info.json`, folder ids and `.log` files.
- The `wry` patch (`src-tauri/patches/wry/`) and the `target/` path used by the `Build apps` workflow.
- Versions in `version.txt`, `package.json`, `tauri.conf.json` and `Cargo.toml`, changed only by a release.

## Folder Structure

### Backend (`src-tauri`)

```
src-tauri/src/
├── lib.rs               App entry and IPC handler registration
├── domain/              Pure model and rules, no IO
├── infra/               Disk IO and persistence
│   ├── storage/         Storage database, info file and recovery
│   └── fs_safe/         Guarded file system operations
├── services/            Use cases
│   ├── mutations/       One module per tree operation, with shared checks
│   ├── index/           Review and search indexes
│   ├── tree/            Tree loading and review links
│   ├── watch/           File watcher and tray events
│   └── ...              Backup, intake, preview, extensions, debug log, updater
├── platform/            OS adapters behind cross-platform wrappers
│   └── windows/         Windows-only implementations
├── app/                 Startup and session state
└── commands/            IPC adapters, grouped by feature
```

#### Rules:

```
commands → services → infra → domain
   app   ↗     ↘ platform
```

- Imports only point down: `domain` imports nothing from the crate's other layers, `infra` only `domain`, `services` `infra` and `domain`.
- `platform` is the only place for OS-specific code; new Windows code goes in `platform/windows/` behind a wrapper with a non-Windows fallback.
- `commands` only validate arguments, call a service and return a DTO; logic found there moves to `services`.
- `lib.rs` keeps `run()` and the `generate_handler!` list; a new command is added there and in `src/services/ipc/api.ts` together.
- Shared helpers live in the lowest layer that uses them: command plumbing in `commands/mod.rs`, mutation checks in `services/mutations/checks.rs`, test setup in `test_support.rs`.

Guard tests:

- `ipc_parity_tests.rs`: the `generate_handler!` names equal the `call("...")` names in `src/services/ipc/api.ts`.
- `layer_tests.rs`: nothing under `domain/` imports `app`, `commands`, `infra`, `platform` or `services`.

### Frontend (`src`)

```
src/
├── main.ts              App bootstrap
├── components/          Atomic design structure
│   ├── atoms/           Single controls and icons
│   ├── molecules/       Small control groups and list items
│   ├── organisms/       Composed UI blocks
│   │   ├── bars/        Toolbar, side and detail panels, status line
│   │   ├── menus/       Context and dropdown menus
│   │   └── dialogs/     Shared dialog shell with info and action bases
│   └── windows/         Pages level
│       ├── views/       Tree, columns, grid and board views
│       └── settings/    Settings window and its pages
├── hooks/               UI behavior and user operations
├── services/            Non-UI logic
│   ├── ipc/             Backend API and types
│   └── store/           App state
├── extensions/          Extension panel and runtime
├── assets/
│   ├── gui/             Interface icons
│   └── icons/           Folder and file icons
└── styles/              Theme, base and per-component stylesheets mirroring components/
```

#### Rules:

```
windows → organisms → molecules → atoms
   ↓          ↓
 hooks  →  services
```

- Components import only lower component levels; `atoms` and `molecules` import no hooks or services.
- `windows` compose organisms into a screen and wire them to hooks; they hold no business logic.
- Only `services/ipc` calls the backend; the rest of the app goes through it.
- A control is always built from its atom or molecule, never by hand in a window or organism.
- Organisms take data and callbacks where it removes coupling (`side-panel`, `board-card`).
- A component's styles live in the matching file under `styles/components/`, one rule per component, no selector lists shared across components. `styles/index.css` imports every sheet in cascade order, and `theme.css` holds only the color variables.
- Icons are recolored at runtime from the "Icon color" setting (`atoms/icons.ts`); the link arrow and overlay marks keep their own colors.

Dialogs: there are only two kinds, both opened through `dialog-shell`. A new dialog is built on one of them.

| Kind | Base | Used for |
| --- | --- | --- |
| Info | `info-dialog/` | A message and a couple of buttons: choices, confirmations, warnings |
| Action | `action-dialog/` | A table or a form: prompts, selects, format validations, loose files |

The settings window is a window, not a dialog: it shares only the `showModal` lifecycle.

### Code style

- TypeScript strict, ESM, `import type` for types; import order framework, external, relative.
- File names kebab-case in `src/`, snake_case in `src-tauri/src/`.
- Files stay small: split past ~150 lines, ~300 is the guideline cap.
- Comments in English, one line, only for the why; each file opens with a 1-4 line header saying why it exists.
- No new dependencies without the owner's approval; pnpm only.

## Running

From `folder-manager/`, with Node through `fnm` and `pnpm.cmd`:

- Dev app on the test storage: `fnm exec --using=24 pnpm.cmd run tauri dev -- -- "<ANT-monorepo>/Testing/Test_Storage"`. Kill any running `folder-manager.exe` and whatever listens on port 1420 first.
- Point `CARGO_TARGET_DIR` at `ANT-on-the-system/Applications/FolderManager/src-tauri/target` to reuse built dependencies, and run one cargo process at a time.
- Never test on `Root_Storage`, and never run the Python and Tauri apps on the same storage.

## Verification

- Backend: `cargo test --locked` from `src-tauri`. The parity and layer tests must pass. `preview::shell::tests::a_text_file_has_an_icon_and_a_thumbnail` hangs on some machines; skip it with `-- --skip` and say so.
- Frontend: `pnpm run typecheck` and `pnpm run build`.
- UI changes: launch the dev app on `Test_Storage` and check the affected screens; build checks don't prove visuals.
- Refactors: the `generate_handler!` list is identical before and after, and each phase is one commit that compiles.
- These checks run before a push of `dev/fmanager`; failures are fixed before pushing.


## Notes

- Services still call `platform` directly; making `platform` reachable only from `app` and `commands` needs dependency injection.
- Some Windows code stays outside `platform/windows` because it depends on service helpers; inline `cfg(windows)` blocks stay where they are.
- Some organisms (toolbar, context menu, details panel) and molecules (name cell, manage row) still import hooks and the store directly.
- `hooks/ops.ts` and `services/store/state.ts` exceed the ~300-line guideline.
- Linux and macOS builds are only verified by CI.
