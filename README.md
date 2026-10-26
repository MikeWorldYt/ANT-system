<div align="center">

# ANT System

**Archive Nesting Technique** is a method and a toolkit for keeping files and folders consistent, searchable and automation-friendly, however large they grow.

[![CreateNewStorage](https://img.shields.io/github/v/release/MikeWorldYt/ANT-system?filter=create-new-storage-*&label=CreateNewStorage&color=00b386)](https://github.com/MikeWorldYt/ANT-system/releases?q=create-new-storage)
[![FolderManager](https://img.shields.io/github/v/release/MikeWorldYt/ANT-system?filter=folder-manager-*&label=FolderManager&color=00b386)](https://github.com/MikeWorldYt/ANT-system/releases?q=folder-manager)
[![Build apps](https://img.shields.io/github/actions/workflow/status/MikeWorldYt/ANT-system/build-apps.yml?branch=main&label=build)](https://github.com/MikeWorldYt/ANT-system/actions/workflows/build-apps.yml)
[![Docs](https://img.shields.io/badge/docs-ant--system.vercel.app-4f8cff)](https://ant-system.vercel.app/en/introduction/what-is-ant/)

[Documentation](https://ant-system.vercel.app/en/introduction/what-is-ant/) ·
[Quick start](#quick-start) ·
[Utilities](#utilities) ·
[Releases](https://github.com/MikeWorldYt/ANT-system/releases) ·
[Contributing](Project'sColaborations/README.md)

</div>

---

## Why ANT?

Folders rot. Names drift, structure depends on whoever made it, and finding something three years later means remembering how you thought back then. ANT fixes this with a small set of strict rules that stay readable at ten files or ten million:

- **Predictable names.** Files and folders follow a naming format (Block, Chill and Control naming), so they sort correctly and can be found with any operating system's search.
- **A fixed shape.** Numbered top-level categories and a DAG-adapted tree give every item one obvious home.
- **Tools that enforce it.** Small utilities create, maintain and rename a storage so the rules cost you nothing to follow.

```
400.ACADEMIC / PJ.033__Thesis-Draft
                │ { 
                │  "id": "03ASGTVCK1H8M6D"
                │  "owner": "Mike Diaz",
                │  "created": "26-10-26",
                │  "status": "in-progress"
                │ }
                └ 
```

## Quick start

Three minutes from nothing to a working storage.

### 1. Create a storage

`CreateNewStorage` is a ~400 KB command-line tool for Windows, Linux and macOS. It builds the whole ANT structure for you.

| Platform | Download |
| --- | --- |
| Windows | [`CreateNewStorage.exe`](https://github.com/MikeWorldYt/ANT-system/releases/download/create-new-storage-v3.1.0/CreateNewStorage.exe) |
| Linux | [`CreateNewStorage-linux.tar.gz`](https://github.com/MikeWorldYt/ANT-system/releases/download/create-new-storage-v3.1.0/CreateNewStorage-linux.tar.gz) |
| macOS | [`CreateNewStorage-macos.tar.gz`](https://github.com/MikeWorldYt/ANT-system/releases/download/create-new-storage-v3.1.0/CreateNewStorage-macos.tar.gz) |

Or from a terminal.

<details open>
<summary><b>Linux</b> (use <code>CreateNewStorage-macos.tar.gz</code> on macOS)</summary>

```bash
curl -L -o cns.tar.gz https://github.com/MikeWorldYt/ANT-system/releases/download/create-new-storage-v3.1.0/CreateNewStorage-linux.tar.gz
tar -xzf cns.tar.gz        # extracts the "create-new-storage" executable, already runnable
./create-new-storage --help
./create-new-storage ~ --name "My-name-storage"   # change the name to yours
```

</details>

<details open>
<summary><b>Windows</b> (PowerShell)</summary>

```powershell
Invoke-WebRequest -Uri https://github.com/MikeWorldYt/ANT-system/releases/download/create-new-storage-v3.1.0/CreateNewStorage.exe -OutFile CreateNewStorage.exe
.\CreateNewStorage.exe --help
.\CreateNewStorage.exe $HOME --name "My-name-storage"   # change the name to yours
```

</details>

`<path>` is where the storage goes and `--name` creates a sanitized subfolder inside it. You get the standard top level:

```
📁 My-Name-Storage
 ├── 📁 100.PERSONAL
 ├── 📁 200.WORK
 ├── 📁 300.PROJECTS
 ├── 📁 400.ACADEMIC
 ├── 📁 500.HOBBIES
 ├── 📁 900.Resources
 ├── 📁 998.Review
 └── 📁 999.Settings
```

### 2. Manage it with FolderManager

`FolderManager` is a desktop app (Tauri 2 and Rust) to browse and organize the storage in tree or column views, rename in place, track status, search and back up, without touching the terminal. Open it from inside the storage, or point it at the folder.

🔽 [Download FolderManager.exe](https://github.com/MikeWorldYt/ANT-system/raw/main/Applications/FolderManager/dist/FolderManager.exe) (Windows; Linux and macOS are in progress)

> Coming from an existing setup? Follow the [Getting Started guide](https://ant-system.vercel.app/en/start-here/getting-started).

## Utilities

| Tool | What it does | Platforms | |
| --- | --- | --- | --- |
| **CreateNewStorage** | Generates a ready-to-use ANT storage from the official template, with the review shortcuts and the starter database. Fast, dependency-free CLI written in Rust. | Windows · Linux · macOS | [Download](https://github.com/MikeWorldYt/ANT-system/releases/tag/create-new-storage-v3.1.0) · [Source](Applications/CreateNewStorage) |
| **FolderManager** | Day-to-day management of a storage: tree and column views, favorites, status colors, details, search, scheduled backups, extensions and a settings window. | Windows | [Download](https://github.com/MikeWorldYt/ANT-system/raw/main/Applications/FolderManager/dist/FolderManager.exe) · [Source](Applications/FolderManager) |
| **Reformatter** | Batch renames existing files to the ANT naming format, with a preview mode, safety filters and automatic skipping of system and hidden files. | Windows | [Download](https://github.com/MikeWorldYt/ANT-system/releases/download/v0.2.1/reformatter.v.0.2.1.exe) · [Source](Applications/Reformatter) |

## Repository layout

```
Applications/       the utilities (CreateNewStorage, FolderManager, Reformatter)
Documentation/      the docs site (Astro + Starlight) and the legacy Angular one
Migration/          the storage template and guides for adopting ANT on an existing setup
Project'sColaborations/   contributor guides
```

Each package is independent, with its own dependencies and no root workspace. Executables are built by the [`Build apps`](.github/workflows/build-apps.yml) workflow and published as one release per app.

## Documentation

Everything about the method lives at **[ant-system.vercel.app](https://ant-system.vercel.app/en/introduction/what-is-ant/)**, in English and Spanish: the naming format, the storage rules, migration plans and how each tool works.

## Contributing

Issues and pull requests are welcome. Read the [contributor guide](Project'sColaborations/README.md) first; changes go through a `dev/<area>` branch and a pull request, and every change tracks an issue.

## Releases

Each app is versioned and released on its own, with the executable attached: [all releases](https://github.com/MikeWorldYt/ANT-system/releases).

---

<div align="center">

Designed by [MikeWorld](https://www.linkedin.com/in/mikeworldyt/)

</div>
