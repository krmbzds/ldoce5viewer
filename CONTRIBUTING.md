# Contributing to LDOCE5 Viewer

Thank you for your interest in contributing!  This guide focuses on the
**non-GUI development workflow** that can be performed without a physical Qt
environment or display server.

---

## Quick start

```bash
# Clone and enter the repo
git clone https://github.com/krmbzds/ldoce5viewer.git
cd ldoce5viewer

# Create a virtual environment
python3 -m venv .venv
source .venv/bin/activate   # Windows: .venv\Scripts\activate

# Install development dependencies (no Qt required)
pip install -r requirements-dev.txt
```

---

## Running the non-GUI test suite

```bash
pytest tests/ -v
```

All tests under `tests/` are headless and do **not** import Qt.  They are safe
to run in CI without a display server.

| Test file | What it covers |
|-----------|----------------|
| `tests/test_cdb.py` | `CDBMaker` / `CDBReader` round-trips |
| `tests/test_incremental.py` | `incremental.Maker` / `Searcher` with a small dataset |
| `tests/test_fulltext.py` | `fulltext.Maker` / `Searcher` / `VariationsReader` using Whoosh |

---

## Linting

```bash
ruff check ldoce5viewer tests
```

Configuration lives in `pyproject.toml` under `[tool.ruff]`.

---

## Making changes

1. **Non-GUI code** (`utils/`, `incremental.py`, `fulltext.py`, `ldoce5/`) —
   always add or update a test that exercises the changed behavior.
2. **GUI code** (`qtgui/`) — changes require manual verification with a running
   Qt desktop environment.  See `README.md` for instructions.
3. Keep commits small and focused on a single logical change.

---

## Regenerating Qt resources (manual, requires PySide6 tools)

See the **Manual steps requiring a local Qt environment** section in `README.md`.

---

## Code style

- Python 3.9+ syntax only; no Python 2 compatibility code.
- Prefer explicit `bytes` / `str` handling; never mix them silently.
- Follow PEP 8 (enforced by `ruff`).
