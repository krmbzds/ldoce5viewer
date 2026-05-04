# LDOCE5 Viewer (PySide6, Python 3, Qt6)

-----

The LDOCE5 Viewer is an alternative dictionary viewer for the Longman Dictionary of Contemporary English 5th Edition (LDOCE 5).

![image](https://cloud.githubusercontent.com/assets/15828926/24585732/efb068a4-17bb-11e7-8294-7241f73d9ed8.png)

It runs on macOS (Intel, arm), Linux and Microsoft Windows.

This software is free and open source software licensed under the terms of GPLv3.

---

## Development

### Prerequisites

- Python 3.9+
- [mise](https://mise.jdx.dev/) (optional but recommended for managing tool versions on macOS)

### Non-GUI tests (CI-safe, no display required)

```bash
# Install dev dependencies
pip install -r requirements-dev.txt

# Run linter
ruff check ldoce5viewer tests

# Run tests (no Qt / display server needed)
pytest tests/ -v
```

All tests under `tests/` are pure-Python and exercise non-GUI subsystems
(CDB, incremental index, fulltext search).  They run without a display server.

---

## Manual steps requiring a local Qt environment

> **These steps require PySide6 tools installed locally and cannot be automated in CI.**

### Regenerating Qt `.py` files from `.ui` and `.qrc` sources

Install PySide6 tools via pip (or your platform package manager):

```bash
pip install PySide6
```

Regenerate UI Python wrappers from `.ui` files:

```bash
# Example for a single .ui file:
pyside6-uic ldoce5viewer/qtgui/ui/main.ui -o ldoce5viewer/qtgui/ui/ui_main.py

# Or use the Makefile target if present:
make ui
```

Regenerate the resource module from `resources.qrc`:

```bash
pyside6-rcc ldoce5viewer/qtgui/resources/resources.qrc \
    -o ldoce5viewer/qtgui/resources/resources_rc.py
```

### Running the GUI interactively

```bash
# Install runtime dependencies
pip install PySide6 lxml Whoosh

# Launch the viewer (requires LDOCE 5 data directory)
python ldoce5viewer.py
```

### arm-based macOS (Apple Silicon) notes

- Use a native arm64 Python (e.g. via `mise` + `pyenv` with `PYTHON_CONFIGURE_OPTS`).
- PySide6 wheels on PyPI are universal2 and work on Apple Silicon without Rosetta.
- If you encounter Qt plugin errors, ensure `PySide6` is installed in the same
  virtual environment as the Python interpreter you are using.

### Troubleshooting Qt on macOS (Homebrew / mise)

```bash
# Verify Qt is found
python -c "from PySide6.QtCore import QCoreApplication; print(QCoreApplication.libraryPaths())"

# If audio fails, ensure the multimedia backend is present:
pip install PySide6-Addons
```

