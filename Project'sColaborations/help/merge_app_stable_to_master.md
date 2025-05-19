# 🔀 How to Safely Merge Stable Changes to Master in a Monorepo

This guide explains how to **merge only specific application updates** from a development branch into a shared `master` branch without affecting other folders or applications in a monorepo structure.

---

## 📁 Scenario

Monorepo structure:

```
/Applications/
  ├── Reformatter/
  └── CreateNewStorage/
```

You are working in a dev branch like:

```
applications/dev/Reformatter
```

And want to push only `Reformatter/` changes to:

```
applications/master
```

---

## ✅ Steps to Promote a Stable App Version

1. **Switch to the master branch**:

```bash
git checkout applications/master
git pull
```

2. **Checkout only the app folder from the dev branch**:

```bash
git checkout applications/dev/Reformatter -- Applications/Reformatter
```

This updates only the `Reformatter` folder from the dev branch, leaving everything else untouched.

3. **Commit the staged changes**:

```bash
git commit -m "feat(reformatter): promote stable changes to master"
```

4. **Push to remote**:

```bash
git push
```

---

## 💡 Notes

- This avoids accidentally modifying or deleting unrelated folders (like `CreateNewStorage/`).
- Works even if other apps exist in the same root folder.
- Only files within the specified subfolder (`Applications/Reformatter/`) will be updated.
- If your local master is behind the remote, use:

```bash
git pull --rebase origin applications/master
```

---

## 🧹 Optional: Remove Other App Folders from Dev Branches

If a dev branch unintentionally includes folders from other apps:

```bash
git rm -r --cached Applications/CreateNewStorage
git commit -m "chore(dev): remove unrelated app from branch"
git push
```

---

Keep your job clean. Promote only what matters.
