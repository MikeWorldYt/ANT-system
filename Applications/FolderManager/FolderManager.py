# PyInstaller entry point for FolderManager; the app lives in the folder_manager package.
import os
import sys
import tkinter as tk

# Source runs need the shared ant_core package one folder up; the frozen exe bundles it
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from folder_manager.app import FolderManagerApp
from folder_manager.updater import check_for_updates


def main():
    start = sys.argv[1] if len(sys.argv) > 1 else os.getcwd()
    root  = tk.Tk()
    app = FolderManagerApp(root, start_path=start)
    exe_path = os.path.abspath(sys.argv[0])
    root.after(1500, lambda: check_for_updates(root, app.root_path, exe_path))
    root.mainloop()


if __name__ == "__main__":
    main()
