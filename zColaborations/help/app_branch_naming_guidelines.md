# 📦 Application Branch Naming & Creation Guidelines

To ensure clean version control and team collaboration within the monorepo, **each new application** must follow a standardized naming and branching structure from the beginning.

---

## 🧭 When to Create a New App Branch

Create a new branch when:

- You are starting an application that does **not yet exist** in `applications/master`
- You are contributing a new tool or CLI utility under `/Applications/`
- You want to develop a proof of concept without affecting others

---

## 📛 Branch Naming Convention

Follow this strict format:

```
applications/dev/<AppName>
```

✅ Where `<AppName>`:

- Must use **CamelCase**
- Must not contain spaces, dashes `-`, or underscores `_`
- Should be descriptive of the app's function
- Must **not** contain personal names

**Examples:**

| Good Examples             | 🚫 Bad Examples               |
|---------------------------|-------------------------------|
| applications/dev/Iconizer | applications/dev/jorge       |
| applications/dev/FileHasher | applications/dev/Change_Icons |
| applications/dev/FavSync  | applications/dev/JorgeTool   |

---

## 🪜 Steps to Create a New App Branch

1. Start from the `applications/master` branch:

```bash
git checkout applications/master
git pull
```

2. Create your new dev branch with a descriptive app name:

```bash
git checkout -b applications/dev/Iconizer
```

3. Inside the `Applications/` folder, create a folder **with the same name in CamelCase**:

```
/Applications/
  └── Iconizer/
```

4. Begin development in your app folder as needed.

5. Commit and push the branch:

```bash
git add .
git commit -m "feat(iconizer): initialize Iconizer app structure"
git push -u origin applications/dev/Iconizer
```

---

## 📁 Folder Naming Rules

- Use **CamelCase**
- No spaces, dashes, or underscores
- Must match the branch name segment

---

## ✅ Summary

| Requirement            | Rule                                   |
|------------------------|----------------------------------------|
| Branch Name            | `applications/dev/<AppName>`          |
| AppName format         | CamelCase, no special characters       |
| Folder Name            | Matches `<AppName>`, also CamelCase   |
| Avoid                 | Personal names in branch or folder     |

---

Keep your job clean. Promote only what matters.
