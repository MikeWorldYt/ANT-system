<div align="center">

# ANT System

**Archive Nesting Technique** is a method system tolkit for keeping files and folders with consistent, searchable and automation-friendly structure, however large they grow.

[![CLI Install](https://img.shields.io/github/v/release/MikeWorldYt/ANT-system?filter=create-new-storage-*&label=CLI-Install&color=00b386)](https://github.com/MikeWorldYt/ANT-system/releases?q=create-new-storage)
[![FolderManager](https://img.shields.io/github/v/release/MikeWorldYt/ANT-system?filter=folder-manager-*&label=FolderManager&color=00b386)](https://github.com/MikeWorldYt/ANT-system/releases?q=folder-manager)

[![Docs](https://img.shields.io/badge/docs-ant.mikeworld.dev-4f8cff)](https://ant.mikeworld.dev/en/introduction/what-is-ant/)
[![Build apps](https://img.shields.io/github/actions/workflow/status/MikeWorldYt/ANT-system/build-apps.yml?branch=main&label=builds)](https://github.com/MikeWorldYt/ANT-system/actions/workflows/build-apps.yml)


[Documentation](https://ant.mikeworld.dev/en/introduction/what-is-ant/) ·
[Quick start](#quick-start) ·
[Utilities](#utilities) ·
[Releases](https://github.com/MikeWorldYt/ANT-system/releases) ·
[Contributing](Project'sColaborations/README.md)

</div>

---

## Why use ANT?

> How many times have you saved something as `Final report updated UPDATED (2).docx` and told yourself ye I'd order it later?... (Later never comes).

> How many times spend time digging amoung thousands of files and folders looking for this same file?

Let's be honest, You often pile up **"important files"** on the `Desktop`, in a folder with the name's client or company, or directly in your `Downloads`. You and ​<u>more than 83% of computer users make it</u>.

Whether you are in a Hurry or don't have idea how name it, You end up saving it as a generic name as `"my project"` in whatever folder opened.

The main app **FolderManager**, tho looks like another File explorer, **it's an automation tool based on ANT System**, a nested, hierarchical categorization rules backed by a `Local Database`;

* **Built-in automation:** the app takes over the tedious work of naming, structuring and sorting, keeping manual work to a minimum.
* **Standardization:** you always know exactly what a file is called and where it lives.
* **Project management:** every project carries a status (in progress, on hold, done) and appears on a built-in Kanban board, so you can review them all in one place without browsing through folders.
* ***AI-ready* architecture:** because your files follow clean, logical rules, your storage becomes a CMS that AI agents can read, process and work with without mistakes.

> A clear digital workspace no requiered paid plan, Whit this **foundations rules for working or studying** without the fatigue of searching for what you already made. It's by far, the smartest way to simplify your day.

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
Documentation/      the docs site (Astro + Starlight)
Migration/          the storage template and guides for adopting ANT on an existing setup
Project'sColaborations/   contributor guides
```

Each package is independent, with its own dependencies and no root workspace. Executables are built by the [`Build apps`](.github/workflows/build-apps.yml) workflow and published as one release per app.

## Documentation

Everything about the method lives at **[ant-system.vercel.app](https://ant-system.vercel.app/en/introduction/what-is-ant/)**, in English and Spanish: the naming format, the storage rules, migration plans and how each tool works.

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

## Contributing

Issues and pull requests are welcome. Read the [contributor guide](Project'sColaborations/README.md) first; changes go through a `dev/<area>` branch and a pull request, and every change tracks an issue.

## Releases

Each app is versioned and released on its own, with the executable attached: [all releases](https://github.com/MikeWorldYt/ANT-system/releases).

---

<div align="center">

Designed by [MikeWorld](https://www.linkedin.com/in/mikeworldyt/)

</div>
