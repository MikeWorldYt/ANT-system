# Folder Manager

Desktop manager for an ANT storage, built with Tauri 2 (Rust backend, TypeScript frontend). The released executable lives in `dist/FolderManager.exe`; `versions.md` lists every release.

## Requirements

- Rust (stable, MSVC) and the WebView2 runtime
- Node 24 and pnpm. Node is managed with `fnm`, so every pnpm command runs through `fnm exec --using=24 pnpm.cmd ...`
- Install once: `pnpm install`

## Develop

Run from this folder. The path after `--` is the storage to open.

```bash
pnpm run tauri dev -- -- "<path to a test storage>"
```

- Use a copy of a storage, never the real one.
- If the dev server dies right after `VITE ready` with `ECONNREFUSED`, remove the Console Ninja folder from `PATH` for that shell.
- A stale `folder-manager.exe` or a listener on port 1420 blocks the next launch; stop both first.
- On low-memory machines limit cargo: `CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0`.

## Check

```bash
pnpm run typecheck
```

```bash
cd src-tauri && cargo test
```

Run one cargo process at a time, and never `cargo test` while `tauri dev` is compiling. If `cargo test` runs out of memory, add `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0`.

## Build

```bash
pnpm run build
```

Builds the frontend into `web-dist/`. The executable comes from:

```bash
pnpm tauri build --no-bundle -- --locked
```

The result is `src-tauri/target/release/folder-manager.exe`.

## Release

1. Bump the version in `version.txt`, `package.json`, `src-tauri/tauri.conf.json` and `src-tauri/Cargo.toml` (and `Cargo.lock` through cargo), and add a row to `versions.md`. The `Build apps` workflow fails if `version.txt` and `tauri.conf.json` disagree.
2. Bring the change into `main` under `Applications/FolderManager/`.
3. The workflow builds the executable, commits it to `dist/` and publishes the `folder-manager-vX.Y.Z` release.

## Layout

- `src/`: frontend, grouped into `ipc`, `store`, `actions`, `ui`, `views` and `shell`. Keyboard shortcuts live in `src/actions/shortcuts.ts`.
- `src-tauri/src/`: backend, grouped into `commands`, `tree`, `index`, `storage`, `mutations`, `fs_safe` and `ant_core`.
- User settings are stored in `<storage>/.ant/ant.db` and survive updates.
