# -*- mode: python ; coding: utf-8 -*-
"""
PyInstaller spec for LDOCE5 Viewer with explicit inclusion of
- project static files
- qtgui resources
- PySide6 Qt plugins (platforms, styles, imageformats, etc.)
- PySide6 Qt libexec (QtWebEngineProcess, etc.)

This helps avoid runtime visual/plugin issues when packaging on macOS.
"""

import glob
import os

block_cipher = None

# Helper: locate PySide6 package to collect Qt plugins/libexec
_pyside_dir = None
try:
    import PySide6

    _pyside_dir = os.path.dirname(PySide6.__file__)
except Exception:
    _pyside_dir = None

# Start with application static data (preserve directory layout)
_datas = []
# include entire ldoce5viewer/static if present
if os.path.isdir(os.path.join("ldoce5viewer", "static")):
    for root, _, files in os.walk(os.path.join("ldoce5viewer", "static")):
        for fn in files:
            src = os.path.join(root, fn)
            rel = os.path.relpath(os.path.join(root, fn), os.path.join("ldoce5viewer", "static"))
            dest = (
                os.path.join("static", os.path.dirname(rel))
                if os.path.dirname(rel) != "."
                else "static"
            )
            _datas.append((os.path.abspath(src), dest))

# include qtgui resources directory
_res_root = os.path.join("ldoce5viewer", "qtgui", "resources")
if os.path.isdir(_res_root):
    for root, _, files in os.walk(_res_root):
        for fn in files:
            src = os.path.join(root, fn)
            rel = os.path.relpath(root, _res_root)
            dest = os.path.join("ldoce5viewer", "qtgui", "resources", rel)
            _datas.append((os.path.abspath(src), dest))

# Collect PySide6 Qt plugins (best-effort)
if _pyside_dir:
    _qt_plugins_dir = os.path.join(_pyside_dir, "Qt", "plugins")
    if os.path.isdir(_qt_plugins_dir):
        for plugin_sub in os.listdir(_qt_plugins_dir):
            subdir = os.path.join(_qt_plugins_dir, plugin_sub)
            if not os.path.isdir(subdir):
                continue
            for f in glob.glob(os.path.join(subdir, "*")):
                # Place plugins under Plugins/<subdir> inside the bundle
                dest = os.path.join("Plugins", plugin_sub)
                _datas.append((os.path.abspath(f), dest))

    # Collect libexec files (QtWebEngineProcess, etc.)
    _qt_libexec = os.path.join(_pyside_dir, "Qt", "libexec")
    if os.path.isdir(_qt_libexec):
        for f in glob.glob(os.path.join(_qt_libexec, "*")):
            # Place libexec files under Frameworks/libexec so they end up in Contents/Frameworks
            dest = os.path.join("Frameworks", "libexec")
            _datas.append((os.path.abspath(f), dest))

# Fallback: ensure at least the top-level static entry is present (placed under 'static' for frozen app)
if not any(d for d in _datas if d[1].startswith("static")):
    _static_src = os.path.join("ldoce5viewer", "static")
    if os.path.isdir(_static_src):
        for root, _, files in os.walk(_static_src):
            for fn in files:
                src = os.path.join(root, fn)
                rel = os.path.relpath(os.path.join(root, fn), _static_src)
                dest = (
                    os.path.join("static", os.path.dirname(rel))
                    if os.path.dirname(rel) != "."
                    else "static"
                )
                _datas.append((os.path.abspath(src), dest))

# Analysis
from PyInstaller.utils.hooks import collect_submodules

a = Analysis(
    ["ldoce5viewer.py"],
    pathex=[],
    binaries=[],
    datas=_datas,
    hiddenimports=[],
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=[],
    win_no_prefer_redirects=False,
    win_private_assemblies=False,
    cipher=block_cipher,
    noarchive=False,
)
pyz = PYZ(a.pure, a.zipped_data, cipher=block_cipher)

exe = EXE(
    pyz,
    a.scripts,
    [],
    exclude_binaries=True,
    name="ldoce5viewer",
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=True,
    console=False,
    disable_windowed_traceback=False,
    argv_emulation=False,
    target_arch=None,
    codesign_identity=None,
    entitlements_file=None,
)
coll = COLLECT(
    exe,
    a.binaries,
    a.zipfiles,
    a.datas,
    strip=False,
    upx=True,
    upx_exclude=[],
    name="ldoce5viewer",
)
app = BUNDLE(
    coll,
    name="LDOCE5 Viewer.app",
    icon="./ldoce5viewer/qtgui/resources/ldoce5viewer.icns",
    bundle_identifier=None,
)
