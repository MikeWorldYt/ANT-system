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

### 1. Install FolderManager

`FolderManager` is the desktop app for Manage your storages: views, rename in place, status tracking, search, automations and backups.

| Platform | Installer | Portable |
| --- | --- | --- |
| Windows | [`FolderManager_installer.exe`](https://github.com/MikeWorldYt/ANT-system/releases/download/folder-manager-v2.5.2/FolderManager_installer.exe) | [`FolderManager_portable.exe`](https://github.com/MikeWorldYt/ANT-system/releases/download/folder-manager-v2.5.2/FolderManager_portable.exe) |
| Linux | [`FolderManager-linux_installer.deb`](https://github.com/MikeWorldYt/ANT-system/releases/download/folder-manager-v2.5.2/FolderManager-linux_installer.deb) | [`FolderManager-linux_portable.tar.gz`](https://github.com/MikeWorldYt/ANT-system/releases/download/folder-manager-v2.5.2/FolderManager-linux_portable.tar.gz) |
| macOS | [`FolderManager-macos_installer.dmg`](https://github.com/MikeWorldYt/ANT-system/releases/download/folder-manager-v2.5.2/FolderManager-macos_installer.dmg) | [`FolderManager-macos_portable.tar.gz`](https://github.com/MikeWorldYt/ANT-system/releases/download/folder-manager-v2.5.2/FolderManager-macos_portable.tar.gz) |

The installer keeps the app up to date by itself. All versions are on the [releases page](https://github.com/MikeWorldYt/ANT-system/releases).

### 2. Create your storage

Open FolderManager and choose **Create new Storage**: pick a folder, give it a name, and the app builds the whole ANT structure for you. A short tour shows the basics the first time.

<p align="center"><img src=".github/assets/readme/quick-start-create-storage.gif" alt="Creating a storage named Mari from the FolderManager welcome screen" width="720"></p>

> Prefer the terminal or need to script it? [`CreateNewStorage`](https://ant-system.vercel.app/en/advanced/cli) builds the same storage from the command line.
>
> Coming from an existing setup? Follow the [Getting Started guide](https://ant-system.vercel.app/en/start-here/getting-started).

## Utilities

| Tool | What it does | Platforms | |
| --- | --- | --- | --- |
| **CreateNewStorage** | Generates a ready-to-use ANT storage from the official template, with the review shortcuts and the starter database. Fast, dependency-free CLI written in Rust. | Windows · Linux · macOS | [Download](https://github.com/MikeWorldYt/ANT-system/releases/tag/create-new-storage-v3.3.0) · [Source](Applications/CreateNewStorage) |
| **FolderManager** | Creates and manages a storage day to day: several views, favorites, status colors, details, search, automations, scheduled backups, extensions and AI integration. | Windows · Linux · macOS | [Download](https://github.com/MikeWorldYt/ANT-system/releases/tag/folder-manager-v2.5.2) · [Source](Applications/FolderManager) |
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

## Code signing policy

Free code signing provided by SignPath.io, certificate by SignPath Foundation.

See the [Code signing policy](https://ant-system.vercel.app/en/reference/code-signing-policy/) for the signed artifacts, roles and privacy statement.

## License

Licensed under AGPL-3.0-only. See [LICENSE](LICENSE), [LICENSING.md](LICENSING.md) and [TRADEMARK.md](TRADEMARK.md).

---

<div align="center">

Designed by [MikeWorld](https://www.linkedin.com/in/mikeworldyt/)

</div>
