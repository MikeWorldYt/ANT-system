# -*- mode: python ; coding: utf-8 -*-

a = Analysis(
    ['FolderManager.py'],
    pathex=[],
    binaries=[],
    datas=[
        ('assets/up.png', 'assets'),
        ('assets/down.png', 'assets'),
        ('assets/plus.png', 'assets'),
        ('assets/refresh.png', 'assets'),
        ('FolderManager.ico', '.')
    ],
    hiddenimports=[
        'yaml', '_yaml',
        'yaml.reader', 'yaml.scanner', 'yaml.parser',
        'yaml.composer', 'yaml.constructor', 'yaml.resolver',
        'yaml.emitter', 'yaml.serializer', 'yaml.representer',
        'yaml.dumper', 'yaml.loader'
    ],
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=[
        'unittest', 'email', 'html', 'http', 'xml', 'xmlrpc',
        'pydoc', 'doctest', 'difflib', 'ftplib', 'imaplib',
        'mailbox', 'mimetypes', 'smtplib', 'poplib', 'telnetlib',
        'sqlite3', 'decimal', 'fractions', 'statistics',
        'multiprocessing', 'concurrent', 'asyncio',
        'logging', 'curses', 'readline', 'rlcompleter',
    ],
    noarchive=False,
)

pyz = PYZ(a.pure)

exe = EXE(
    pyz,
    a.scripts,
    a.binaries,
    a.datas,
    [],
    name='FolderManager',
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=False,
    upx_exclude=[],
    runtime_tmpdir=None,
    console=False,
    disable_windowed_traceback=False,
    version='version.txt',
    icon='FolderManager.ico'
)
