from PySide6.QtCore import Qt
from PySide6.QtGui import QGuiApplication, QPalette


def is_dark_mode():
    """Detect whether the system is in dark mode.

    Checks ``QStyleHints.colorScheme()`` first (Qt 6.5+), then falls back to
    checking the palette window background luminance for older Qt versions.
    """
    hints = QGuiApplication.styleHints()
    if hasattr(hints, "colorScheme"):
        try:
            return hints.colorScheme() == Qt.ColorScheme.Dark
        except AttributeError:
            pass
    # Fallback: check palette window background luminance
    bg = QGuiApplication.palette().color(QPalette.ColorRole.Window)
    return bg.lightness() < 128
