"""Entry point for the application"""

import codecs
import logging
import os.path
import sys
from optparse import OptionParser

from PySide6.QtGui import QIcon

# set a dummy function if QLineEdit doesn't have setPlaceholderText
from PySide6.QtWidgets import QLineEdit

from .. import __author__
from .config import get_config
from .utils.error import MyStreamHandler, StdErrWrapper
from .utils.singleapp import SingleApplication

_SINGLEAPP_KEY = "ed437af1-0388-4e13-90e9-486bdc88c77a"

if not hasattr(QLineEdit, "setPlaceholderText"):

    def _dummySetPlaceholderText(self, *args, **kwargs):
        pass

    setattr(QLineEdit, "setPlaceholderText", _dummySetPlaceholderText)


# Ensure generated UI and resource python files exist when running from source
# (this helps developers who forget to run the precompile step).
if not getattr(sys, "frozen", False):
    try:
        _here = os.path.dirname(__file__)
        _ui_dir = os.path.join(_here, "ui")
        _res_dir = os.path.join(_here, "resources")
        _need_gen = False
        # check for at least the main generated ui and resources module
        if not os.path.exists(os.path.join(_ui_dir, "main.py")):
            _need_gen = True
        if not os.path.exists(os.path.join(_res_dir, "__init__.py")):
            _need_gen = True
        if _need_gen:
            print("Generating Qt .py UI wrappers and resources (pyside6-uic / pyside6-rcc)...")
            # Prefer pyside6 module entrypoints, fallback to CLI on PATH
            import shutil
            import subprocess

            pyside6_uic = None
            pyside6_rcc = None
            try:
                import importlib

                if importlib.util.find_spec("PySide6.scripts.uic"):
                    pyside6_uic = [sys.executable, "-m", "PySide6.scripts.uic"]
                if importlib.util.find_spec("PySide6.scripts.rcc"):
                    pyside6_rcc = [sys.executable, "-m", "PySide6.scripts.rcc"]
            except Exception:
                pass
            if pyside6_uic is None:
                uic_bin = shutil.which("pyside6-uic")
                if uic_bin:
                    pyside6_uic = [uic_bin]
            if pyside6_rcc is None:
                rcc_bin = shutil.which("pyside6-rcc")
                if rcc_bin:
                    pyside6_rcc = [rcc_bin]

            if pyside6_uic is None or pyside6_rcc is None:
                raise RuntimeError(
                    "PySide6 uic/rcc not found in environment. Please install PySide6 and ensure pyside6-uic and pyside6-rcc are on PATH or available as module entrypoints."
                )

            # Generate all .ui files
            for ui_file in os.listdir(_ui_dir):
                if ui_file.endswith(".ui"):
                    src = os.path.join(_ui_dir, ui_file)
                    dst = os.path.join(_ui_dir, ui_file[:-3] + "py")
                    cmd = pyside6_uic + [src, "-o", dst]
                    subprocess.check_call(cmd)
            # Generate resources
            qrc = os.path.join(_res_dir, "resource.qrc")
            if os.path.exists(qrc):
                dst = os.path.join(_res_dir, "__init__.py")
                cmd = pyside6_rcc + [qrc, "-o", dst]
                subprocess.check_call(cmd)
            print("Generation complete.")
    except Exception as _e:
        # If generation failed, raise a helpful ImportError so the user sees instructions
        raise ImportError(
            "Failed to generate Qt UI/resources: {0}.\nPlease run 'pyside6-uic' and 'pyside6-rcc' manually or run 'make precompile'.".format(
                _e
            )
        )

# Finally import package resources and ui (ui __init__ no longer eagerly imports submodules)
from . import resources, ui  # noqa: E402, F401


def _setup_py2exe(config):
    # suspend py2exe's logging facility
    log_path = os.path.join(config._config_dir, "log.txt")
    try:
        f = codecs.open(log_path, "w", encoding="utf-8")
    except Exception:
        pass
    else:
        sys.stderr = f


def run(argv):
    """start the application"""

    config = get_config()

    # py2exe
    if sys.platform == "win32" and (hasattr(sys, "frozen") or hasattr(sys, "importers")):
        _setup_py2exe(config)

    # Parse arguments
    optparser = OptionParser()
    optparser.set_defaults(debug=False)
    optparser.add_option("--debug", action="store_true", help="Enable debug mode")
    (options, args) = optparser.parse_args(argv)

    # stderr wrapper
    sys.stderr = StdErrWrapper(sys.stderr)

    # logging
    logger = logging.getLogger()
    handler = MyStreamHandler()
    handler.setFormatter(logging.Formatter("%(levelname)s:%(name)s:%(message)s"))
    logger.addHandler(handler)
    logger.setLevel(logging.DEBUG if options.debug else logging.ERROR)

    # Create an application instance
    app = SingleApplication(argv, _SINGLEAPP_KEY)
    if app.isRunning():
        app.sendMessage("activate")
        return 1

    # Load the configuration file
    config.debug = options.debug
    config.load()

    # Set the application's information
    app.setApplicationName(config.app_name)
    app.setOrganizationName(__author__)
    app.setWindowIcon(QIcon(":/icons/icon.png"))

    # Setup MainWindow
    from .main import MainWindow

    main_window = MainWindow()

    def messageHandler(msg):
        if msg == "activate":
            main_window.activateWindow()
            main_window.setVisible(True)

    app.messageAvailable.connect(messageHandler)

    # On Windows-ja
    if app.font().family() == "MS UI Gothic":
        cand = (("Segoe UI", None), ("Meiryo UI", None), ("Tahoma", 8))
        from PySide6.QtGui import QFont

        for name, point in cand:
            ps = app.font().pointSize()
            if point is None:
                point = ps if ps != -1 else 9
            font = QFont(name, point)
            if font.exactMatch():
                app.setFont(font)
                break

    # Redirect stderr to the Error Console
    if not options.debug:
        sys.stderr.setApplication(app)

    # Start the application
    r = app.exec()

    # Quit
    config.save()
    return r
