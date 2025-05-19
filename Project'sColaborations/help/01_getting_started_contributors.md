# 🚀 Getting Started with Contributions

Thank you for your interest in contributing to the ANT system!  
To maintain organization and safety in the project, every contributor must follow these steps to start properly.

---

## 🧭 1. Fork the Repository

To work in isolation without affecting the main project:

1. Go to: [https://github.com/MikeWorldYt/ANT-system](https://github.com/MikeWorldYt/ANT-system)
2. Click on the **Fork** button (top-right corner)
3. This will create a personal copy of the repository in your account

---

## ⬇️ 2. Clone Your Fork

On your local machine:

```bash
git clone https://github.com/<YOUR_USERNAME>/ANT-system.git
cd ANT-system
```

---

## 🌱 3. Create a Dev Branch for Your Application

Follow the naming convention. Example if you're building an app called `Iconizer`:

```bash
git checkout -b applications/dev/Iconizer
```

**⚠️ Important:** Do not use personal names or dashes. More info:  
[`app_branch_naming_guidelines.md`](./02_app_branch_naming_guidelines.md)

---

## 🛠️ 4. Start Working on Your App

Place your code in:

```
/Applications/Iconizer/
```

Make sure to use proper folder naming (CamelCase, no dashes, etc.)

---

## 📤 5. Push Your Branch

```bash
git add .
git commit -m "feat(iconizer): initial version"
git push origin applications/dev/Iconizer
```

---

## 🔁 6. Open a Pull Request (PR)

1. Go to your fork on GitHub
  
2. Click on **Compare & Pull Request**
  
3. Make sure you're targeting:  
  `MikeWorldYt/ANT-system` and base branch is `applications/master`
  
4. In the PR message include:
  
  - A brief description of what your app does
  - A list of features or brief description about your app
  - Links to any relevant documentation or resources

5. Click on **Create Pull Request**

---

## ✅ Your PR will be reviewed if:

- You follow the [naming guidelines](./02_app_branch_naming_guidelines.md)
- Your folder is well structured
- You did not affect other apps
- Your branch contains only your application

---

Thank you for contributing in an organized way! 🎉