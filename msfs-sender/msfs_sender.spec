# PyInstaller spec for building standalone Windows .exe
# Run: pyinstaller msfs_sender.spec

a = Analysis(
    ['msfs_sender/__main__.py'],
    pathex=[],
    binaries=[],
    datas=[],
    hiddenimports=['SimConnect'],
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
    name='msfs-gyro-sender',
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=True,
    console=True,
    icon=None,
)
