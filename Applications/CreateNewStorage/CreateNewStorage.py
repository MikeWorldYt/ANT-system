# PyInstaller entry point for the ANT new-storage wizard; the app lives in the create_new_storage package.
import os
import sys
import tkinter as tk

# Source runs need the shared ant_core package one folder up; the frozen exe bundles it
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from create_new_storage.wizard import ANTWizard


def main():
    root = tk.Tk()
    ANTWizard(root)
    root.mainloop()


if __name__ == "__main__":
    main()
