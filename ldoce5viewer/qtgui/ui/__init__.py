# ui package
# Avoid importing submodules at package import time to prevent circular imports.
# Consumers should import specific submodules, e.g. `from ldoce5viewer.qtgui.ui import main` or
# `from ldoce5viewer.qtgui.ui.main import Ui_MainWindow`.

__all__ = [
    "advanced",
    "indexer",
    "main",
    "custom",
]
