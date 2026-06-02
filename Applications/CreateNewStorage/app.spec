# -*- mode: python ; coding: utf-8 -*-

a = Analysis(
    ['CreateNewStorage.py'],
    pathex=[],
    binaries=[],
    datas=[
        ('CreateNewStorage.ico', '.'),
        ('assets/ant.png', 'assets'),
    ],
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=[],
    noarchive=False,
)

pyz = PYZ(a.pure)

exe = EXE(
    pyz,
    a.scripts,
    a.binaries,
    a.datas,
    [],
    name='CreateNewStorage',
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=True,
    upx_exclude=[],
    runtime_tmpdir=None,
    console=False,
    disable_windowed_traceback=False,
    version='version.txt',
    icon='CreateNewStorage.ico'
)
