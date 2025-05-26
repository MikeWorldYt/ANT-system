# 📂 ANT Reformatter

**ANT Reformatter** is a standalone desktop tool that lets you **batch rename files** using a **customizable, modular naming template**. It was designed for structured file organization using the **ANT (Archive Nesting Technique)** system.

Whether you're managing documents, media files, or project assets, this tool helps you keep everything consistently named and easy to sort.

---

## ✨ Features

- ✅ Rename multiple files in bulk based on custom format.
- 🧱 Modular components: toggle and configure each field.
- 🔍 Live preview of the final filename format.
- 🧠 Auto-resolves folder names and structure using breadcrumb logic.
- 🚫 Skips temporary/system files automatically.
- 🎛️ User-friendly interface.

---

## 🚀 Getting Started

### ✅ Download the App

You can download the latest version of **ANT Reformatter** directly from the Releases page:

📦 **[Download v0.2.1](https://github.com/MikeWorldYt/ANT-system/releases/tag/v0.2.1)**

No installation, Python, or setup is required — just download and run the `.exe` file.

### ▶️ How to Run

1. Unzip the downloaded file (if it's in a `.zip` archive).
2. Double-click the executable file: `ANT_Reformatter.exe`.
3. Application will launch immediately.

---

## 🧭 How to Use

1. **Choose a folder** with the files you want to rename, then **ok**.
2. When load the second window:
   - Enable or disable different parts of the naming template.
   - Configure each section:
     - 📅 `Date`: Add a custom date (by default its their current date).
     - 🧭 `Breadcrumb`: Insert parent folder names (by default load selected folder name).
     - 🏷️ `Holder`: Add a static text or label.
     - 🔢 `Identifier`: Add a unique number or text.
     - 📝 `Postformat`: Append any extra text.
3. See the **live filename preview** update as you edit.
4. When ready, click **Rename Files**.
5. Wait for the status message to confirm completion.

---

## 📁 Example Output

Original filename:  
```
report_final_version.pdf
```

Renamed to:  
```
2024-05_ProjectA_Q1_Report_v2.pdf
```

*(Assuming selected sections are: Date, Breadcrumb, Identifier, and Postformat)*

---

## 📌 Notes

- Files that begin with `~` or match the script name will be ignored.
- Includes a progress bar and status message to indicate completion.
- A `desktop.ini` or custom icons can be handled manually if needed later.

---

## 📜 License

MIT License

---

## 🤝 Contributing

Feel free to fork the project and submit pull requests. Guidelines will be provided soon.

---

## 📬 Contact

Questions or suggestions? Reach out at [mikeworldyt@gmail.com]
