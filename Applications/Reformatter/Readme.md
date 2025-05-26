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
   - Configure each field:
     - 📅 `Date`: Add a custom date (by default its their current date).
     - 🧭 `Breadcrumb`: Insert parent folder names (by default load selected folder name).
     - 🏷️ `Holder`: Add a holder name or label.
     - 🔢 `Identifier`: Add a identifier (by default its `$ID`).
     - 📝 `Postformat`: Append any extra text.
3. See the **filename preview** at the top, meanwhile you edit the fields.
4. If **filename preview** for any reason is not correct, click on **Update All**
5. When ready, click **Start batch**.
6. A status message will appear, Ensure all information is correctly, then click **OK**.
7. Wait the application rename the files with the new format.
8. When done, click **OK**.
9. If you want to rename other files, just repeat the process, if is another folder click on **Back**.

[Identifier List Guide](https://docs.google.com/spreadsheets/d/e/2PACX-1vTXWGayu4tMQb1W1HtCT-tmRGfwDyoRkHh1Lfsc20i9s4lNmRB_X2TTRkjC16WalbuJ9spkqwjPj019/pubhtml)

---

## 📁 Example Output

Original filename:  
```
report-final-version.pdf
```

Renamed to:  
```
(25-06-24) Stocktaking [Amazon]_DOC_Q2 report-final-version.pdf
```

*(Assuming selected sections are: Date, Breadcrumb, Identifier, and Postformat)*
*Date: 25-06-24*
*Breadcrumb: Amazon*
*Identifier: DOC*
*Postformat: Q2*

---

## 📌 Notes

- Files that begin with `~`, `.`, `#`, `@`, `!` or match the script name will be ignored.
- Includes a progress bar and status message to indicate completion.
- The app detect OS files and ignored them, so as not corrupt the system (e.g. `desktop.ini`).

---

## 📜 License

This software is licensed under a custom non-commercial license.  
You are free to use, modify, and share it for non-commercial purposes.  
**Commercial use requires prior written permission.**  
See the [LICENSE.txt](https://github.com/MikeWorldYt/ANT-system/blob/main/Applications/Reformatter/Licence.md) file for full terms.

---

## 🤝 Contributing

Feel free to fork the project and submit pull requests. Guidelines will be provided soon.

---

## 📬 Contact

Questions or suggestions? Reach out at [mikeworldyt@gmail.com]
