# 🐜 ANT System — Monorepo Structure

As of **May 2025**, this repository has transitioned into a **modular monorepo** format to support the growing ecosystem of tools, scripts, and documentation under a single structure.

## 💻 Contributing

Before you start, please make sure to read our [Contributing Guidelines](https://github.com/MikeWorldYt/ANT-system/wiki/CONTRIBUTING).

👨‍💻 When working on a specific tool or site:

- Navigate to its folder and work within that scope
- Commit to the appropriate `dev-*` branch
- Merge to the module’s `master` when ready
- `main` collects the complete structure for deployment or syncing


## 📁 Project Structure

```
/Applications/
  ├── Reformatter/         # Tool for batch file formatting
  └── CreateNewStorage/    # Upcoming Script

/Documentation/
  ├── Angular/             # Legacy website (currently in use)
  └── Astro/               # Future replacement for doc website
```

## 🧩 About the Monorepo Layout

Each module or subproject now has its own folder and can evolve independently.

- **Applications** — CLI tools, Programs or other utilities
- **Documentation** — Public-facing documentation websites

## 🔀 Branching Strategy

| Folder | Development Branches | Stable Branch |
| --- | --- | --- |
| `/Applications` | `dev-reformatter`, etc. | `applications/master` |
| `/Documentation` | `dev-angular`, `dev-astro` | `documentation/master` |
| Entire Repo | `dev`, feature branches | `main` (default) |

A legacy version of the pre-monorepo code is preserved in the [`main-legacy`](https://github.com/MikeWorldYt/ANT-system/tree/main-legacy) branch.

---

Welcome to the new era of ANT System 🐜 — organized, scalable, and modular.