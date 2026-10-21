# Builds the FolderManager window: toolbar, tree list and the actions panel.
import os
import tkinter as tk
from tkinter import ttk

from ant_core.theme import (
    BG, TEXT, TITLE, BUTTON, INPUT_BG, TREE_BG, HIGHLIGHT, DISABLED_TEXT, BORDER,
    HOVER_BTN, DELETE_BTN, DELETE_HOVER,
)

from .config import APP_DIR, TYPE_DISPLAY, STATUS_OPTIONS


class UiMixin:
    def _build_ui(self):
        header = tk.Frame(self.tk_root, bg=BG, pady=6)
        header.pack(fill=tk.X, padx=12)
        tk.Label(header, text=f"Storage Root Name : {os.path.basename(self.root_path)}",
                 bg=BG, fg=TITLE, font=("Segoe UI", 11, "bold")).pack(side=tk.LEFT)

        # ── Toolbar icons ──────────────────────────────────────────
        script_dir = os.path.join(APP_DIR, "assets")
        def _load_icon(filename):
            try:
                return tk.PhotoImage(file=os.path.join(script_dir, filename))
            except tk.TclError:
                # Missing icon: the button falls back to its text label
                return None

        self._ico_up      = _load_icon("up.png")
        self._ico_down    = _load_icon("down.png")
        self._ico_plus    = _load_icon("plus.png")
        self._ico_refresh = _load_icon("refresh.png")

        btn_cfg = dict(bg=BG, relief=tk.FLAT, bd=0,
                       activebackground=INPUT_BG, cursor="hand2", padx=4)

        tk.Label(header, text="Actions", bg=BG, fg=TITLE,
                   font=("Segoe UI", 11, "bold")).pack(side=tk.RIGHT, padx=(90, 80))

        self.btn_refresh = tk.Button(
            header, image=self._ico_refresh, text="↺" if not self._ico_refresh else "",
            command=self._refresh, **btn_cfg)
        self.btn_refresh.pack(side=tk.RIGHT, padx=(12, 0))

        self.btn_move_up = tk.Button(
            header, image=self._ico_up, text="▲" if not self._ico_up else "",
            command=self._move_up, **btn_cfg)
        self.btn_move_up.pack(side=tk.RIGHT, padx=(2, 0))

        self.btn_move_down = tk.Button(
            header, image=self._ico_down, text="▼" if not self._ico_down else "",
            command=self._move_down, **btn_cfg)
        self.btn_move_down.pack(side=tk.RIGHT, padx=(2, 0))

        self.btn_add = tk.Button(
            header, image=self._ico_plus, text="+" if not self._ico_plus else "",
            command=self._add_node, **btn_cfg)
        self.btn_add.pack(side=tk.RIGHT, padx=(2, 0))

        main = tk.Frame(self.tk_root, bg=BG)
        main.pack(fill=tk.BOTH, expand=True, padx=12, pady=(0, 12))

        tree_frame = tk.Frame(main, bg=TREE_BG)
        tree_frame.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)
        self._build_tree_panel(tree_frame)

        actions_frame = tk.Frame(main, bg=INPUT_BG, width=220)
        actions_frame.pack(side=tk.RIGHT, fill=tk.Y, padx=(10, 0))
        actions_frame.pack_propagate(False)
        self._build_actions_panel(actions_frame)

    def _build_tree_panel(self, parent):
        sb = tk.Scrollbar(parent, orient=tk.VERTICAL, bg=BORDER, troughcolor=TREE_BG)
        sb.pack(side=tk.RIGHT, fill=tk.Y)
        self.listbox = tk.Listbox(
            parent, bg=TREE_BG, fg=TEXT,
            selectbackground=HIGHLIGHT, selectforeground=TEXT,
            font=("Segoe UI", 10), bd=0, highlightthickness=0,
            activestyle="none", yscrollcommand=sb.set, cursor="arrow",
        )
        self.listbox.pack(fill=tk.BOTH, expand=True)
        sb.config(command=self.listbox.yview)
        self.listbox.bind("<<ListboxSelect>>", self._on_select)
        self.listbox.bind("<Double-Button-1>", self._on_double_click)
        self.listbox.bind("<Return>", self._on_double_click)

    def _build_actions_panel(self, parent):
        # Name
        name_row = tk.Frame(parent, bg=INPUT_BG)
        name_row.pack(fill=tk.X, padx=12, pady=(18, 0))
        tk.Label(name_row, text="Name:", bg=INPUT_BG, fg=TEXT,
                 font=("Segoe UI", 10), width=7, anchor="w").pack(side=tk.LEFT)
        self.rename_entry = tk.Entry(
            name_row, bg="#3a3a3a", fg=TEXT, insertbackground=TEXT,
            font=("Segoe UI", 10), bd=0, relief=tk.FLAT,
            highlightthickness=1, highlightcolor=BUTTON, highlightbackground=BORDER,
        )
        self.rename_entry.pack(side=tk.LEFT, fill=tk.X, expand=True, ipady=4)

        # Type
        type_row = tk.Frame(parent, bg=INPUT_BG)
        type_row.pack(fill=tk.X, padx=12, pady=(10, 0))
        tk.Label(type_row, text="Type:", bg=INPUT_BG, fg=TEXT,
                 font=("Segoe UI", 10), width=7, anchor="w").pack(side=tk.LEFT)

        self.type_var = tk.StringVar(value=TYPE_DISPLAY[0])
        style = ttk.Style()
        style.theme_use("clam")
        style.configure("Dark.TCombobox",
            fieldbackground="#3a3a3a", background=BORDER, foreground=TEXT,
            selectbackground="#3a3a3a", selectforeground=TEXT,
            arrowcolor=TEXT, bordercolor=BORDER,
            lightcolor=BORDER, darkcolor=BORDER, padding=(4, 2),
        )
        style.map("Dark.TCombobox",
            fieldbackground=[("disabled", INPUT_BG), ("readonly", "#3a3a3a")],
            foreground=[("disabled", DISABLED_TEXT), ("readonly", TEXT)],
            arrowcolor=[("disabled", DISABLED_TEXT), ("readonly", TEXT)],
            bordercolor=[("disabled", BORDER), ("readonly", BORDER)],
        )
        self.type_combo = ttk.Combobox(
            type_row, textvariable=self.type_var,
            values=TYPE_DISPLAY, state="readonly",
            font=("Segoe UI", 10), style="Dark.TCombobox",
        )
        self.type_combo.pack(side=tk.LEFT, fill=tk.X, expand=True, ipady=2)
        self.type_combo.bind("<<ComboboxSelected>>", self._on_type_changed)

        # Status row (hidden by default, shown for WKB)
        self.status_row = tk.Frame(parent, bg=INPUT_BG)
        tk.Label(self.status_row, text="Status:", bg=INPUT_BG, fg=TEXT,
                 font=("Segoe UI", 10), width=7, anchor="w").pack(side=tk.LEFT)
        self.status_var = tk.StringVar(value=STATUS_OPTIONS[0])
        self.status_combo = ttk.Combobox(
            self.status_row, textvariable=self.status_var,
            values=STATUS_OPTIONS, state="readonly",
            font=("Segoe UI", 10), style="Dark.TCombobox",
        )
        self.status_combo.pack(side=tk.LEFT, fill=tk.X, expand=True, ipady=2)

        # Apply button
        self.rename_btn = tk.Button(
            parent, text="Apply", bg=BUTTON, fg=BG,
            font=("Segoe UI", 10, "bold"), bd=0, relief=tk.FLAT,
            activebackground=HOVER_BTN, activeforeground=BG,
            cursor="hand2", pady=8, command=self._apply_rename,
        )
        self.rename_btn.pack(fill=tk.X, padx=12, pady=(14, 0))

        # Delete button
        self.delete_btn = tk.Button(
            parent, text="Delete", bg=DELETE_BTN, fg=TEXT,
            font=("Segoe UI", 10, "bold"), bd=0, relief=tk.FLAT,
            activebackground=DELETE_HOVER, activeforeground=TEXT,
            cursor="hand2", pady=8, command=self._apply_delete,
        )
        self.delete_btn.pack(fill=tk.X, padx=12, pady=(6, 0))

        # Status label
        self.status_label = tk.Label(
            parent, text="", bg=INPUT_BG, fg=DISABLED_TEXT,
            font=("Segoe UI", 8), anchor="w", wraplength=196, justify=tk.LEFT,
        )
        self.status_label.pack(fill=tk.X, padx=12, pady=(8, 0))

        self._set_actions_enabled(False)

    def _set_actions_enabled(self, enabled: bool):
        state       = tk.NORMAL if enabled else tk.DISABLED
        combo_state = "readonly" if enabled else tk.DISABLED
        self.rename_entry.config(
            state=state,
            bg="#3a3a3a" if enabled else INPUT_BG,
            fg=TEXT if enabled else DISABLED_TEXT,
            disabledbackground=INPUT_BG,
            disabledforeground=DISABLED_TEXT,
        )
        self.type_combo.config(state=combo_state)
        self.status_combo.config(state=combo_state)
        self.rename_btn.config(
            state=state,
            bg=BUTTON if enabled else BORDER,
            fg=BG if enabled else DISABLED_TEXT,
            cursor="hand2" if enabled else "arrow",
        )
        self.delete_btn.config(
            state=state,
            bg=DELETE_BTN if enabled else BORDER,
            fg=TEXT if enabled else DISABLED_TEXT,
            cursor="hand2" if enabled else "arrow",
        )
        if not enabled:
            self._hide_status_row()
