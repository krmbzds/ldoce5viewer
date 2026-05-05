import sys

from PySide6.QtCore import QSize, Qt, Signal
from PySide6.QtGui import QAction, QColor, QIcon, QKeySequence, QPalette, QTextDocument
from PySide6.QtWebEngineCore import QWebEnginePage
from PySide6.QtWebEngineWidgets import QWebEngineView
from PySide6.QtWidgets import (
    QApplication,
    QLineEdit,
    QListWidget,
    QStyle,
    QStyledItemDelegate,
    QStyleOptionToolButton,
    QStylePainter,
    QToolButton,
)

from ...utils.text import ellipsis
from ..utils import is_dark_mode as _is_dark_mode

DisplayRole = Qt.ItemDataRole.DisplayRole
State_Selected = QStyle.StateFlag.State_Selected


_BG_COLOR_DARK = "#1e1e1e"
_BG_COLOR_LIGHT = "white"


class ToolButton(QToolButton):
    """QToolButton without menu-arrow"""

    def paintEvent(self, event):
        opt = QStyleOptionToolButton()
        self.initStyleOption(opt)
        opt.features &= ~QStyleOptionToolButton.ToolButtonFeature.HasMenu
        painter = QStylePainter(self)
        painter.drawComplexControl(QStyle.ComplexControl.CC_ToolButton, opt)

    def sizeHint(self):
        opt = QStyleOptionToolButton()
        self.initStyleOption(opt)
        opt.features &= ~QStyleOptionToolButton.ToolButtonFeature.HasMenu
        content_size = opt.iconSize
        return self.style().sizeFromContents(
            QStyle.ContentsType.CT_ToolButton, opt, content_size, self
        )


class LineEdit(QLineEdit):
    """QLineEdit with a clear button"""

    _ICONSIZE = 16

    def __init__(self, parent=None):
        super(LineEdit, self).__init__(parent)
        ICONSIZE = self._ICONSIZE

        self._buttonFind = QToolButton(self)
        self._buttonFind.setCursor(Qt.CursorShape.ArrowCursor)
        self._buttonFind.setIconSize(QSize(ICONSIZE, ICONSIZE))
        self._buttonFind.setIcon(QIcon(":/icons/edit-find.png"))
        self._buttonFind.setStyleSheet("QToolButton { border: none; margin: 0; padding: 0; }")
        self._buttonFind.setFocusPolicy(Qt.FocusPolicy.NoFocus)
        self._buttonFind.clicked.connect(self.selectAll)

        self._buttonClear = QToolButton(self)
        self._buttonClear.hide()
        self._buttonClear.setToolTip("Clear")
        self._buttonClear.setCursor(Qt.CursorShape.ArrowCursor)
        self._buttonClear.setIconSize(QSize(ICONSIZE, ICONSIZE))
        self._buttonClear.setIcon(QIcon(":/icons/edit-clear.png"))
        self._buttonClear.setStyleSheet("QToolButton { border: none; margin: 0; padding: 0; }")
        self._buttonClear.setFocusPolicy(Qt.FocusPolicy.NoFocus)
        self._buttonClear.clicked.connect(self.clear)

        minsize = self.minimumSizeHint()
        framewidth = self.style().pixelMetric(QStyle.PixelMetric.PM_DefaultFrameWidth)
        margin = self.textMargins()
        margin.setLeft(3 + ICONSIZE + 1)
        margin.setRight(1 + ICONSIZE + 3)
        self.setTextMargins(margin)

        height = max(minsize.height(), ICONSIZE + (framewidth + 2) * 2)
        self.setMinimumSize(
            max(minsize.width(), (ICONSIZE + framewidth + 2 + 2) * 2),
            int(height / 2.0 + 0.5) * 2,
        )

        self.textChanged.connect(self.__onTextChanged)

    def resizeEvent(self, event):
        ICONSIZE = self._ICONSIZE
        framewidth = self.style().pixelMetric(QStyle.PixelMetric.PM_DefaultFrameWidth)
        rect = self.rect()
        self._buttonFind.move(framewidth + 3 - 1, (rect.height() - ICONSIZE) / 2 - 1)
        self._buttonClear.move(
            rect.width() - framewidth - 3 - ICONSIZE - 1,
            (rect.height() - ICONSIZE) / 2 - 1,
        )

    def __onTextChanged(self, text):
        self._buttonClear.setVisible(bool(text))


class LineEditFind(QLineEdit):
    shiftReturnPressed = Signal()
    escapePressed = Signal()

    def __init__(self, parent):
        super(LineEditFind, self).__init__(parent)

    def keyPressEvent(self, event):
        if event.key() == Qt.Key.Key_Escape:
            self.escapePressed.emit()
        elif (
            event.key() == Qt.Key.Key_Return
            and event.modifiers() == Qt.KeyboardModifier.ShiftModifier
        ):
            self.shiftReturnPressed.emit()
        elif event.key() == Qt.Key.Key_Return:
            self.returnPressed.emit()
        else:
            super(LineEditFind, self).keyPressEvent(event)


class HtmlListWidget(QListWidget):
    class HtmlItemDelegate(QStyledItemDelegate):
        MARGIN_H = 5
        if sys.platform.startswith("win"):
            MARGIN_V = 3
        elif sys.platform.startswith("darwin"):
            MARGIN_V = 4
        else:
            MARGIN_V = 5

        def __init__(self, parent=None):
            super(HtmlListWidget.HtmlItemDelegate, self).__init__(parent)
            self._doc = QTextDocument()
            self._doc.setDocumentMargin(0)
            self._item_size = None

        def paint(self, painter, option, index):
            doc = self._doc
            painter.resetTransform()
            rect = option.rect
            if option.state & State_Selected:
                highlight = option.palette.color(QPalette.ColorRole.Highlight)
                painter.fillRect(rect, highlight)
            doc.setHtml(index.data(DisplayRole))
            px = rect.x() + self.MARGIN_H
            py = rect.y() + self.MARGIN_V
            painter.translate(px, py)
            doc.drawContents(painter)

        def sizeHint(self, option, index):
            """
            Estimate item height quickly using font metrics and a plain-text
            approximation of the HTML content to avoid expensive QTextDocument
            layout during list sizing. Cache the result per-delegate.
            """
            s = self._item_size
            if not s:
                try:
                    # Get plain text by stripping simple HTML tags
                    import re

                    text_html = index.data(DisplayRole) or ""
                    text_plain = re.sub(r"<[^>]+>", "", text_html)

                    fm = option.fontMetrics
                    avg_char_w = max(1, fm.averageCharWidth())
                    available_width = max(80, option.rect.width() - self.MARGIN_H * 2)
                    chars_per_line = max(20, int(available_width / avg_char_w))
                    import math

                    lines = max(1, int(math.ceil(len(text_plain) / chars_per_line)))
                    # Ensure at least a couple of lines to avoid clipping
                    lines = max(lines, 2)
                    height = fm.lineSpacing() * lines + self.MARGIN_V * 2
                    s = self._item_size = QSize(0, int(height))
                except Exception:
                    # Fallback conservative estimate
                    s = self._item_size = QSize(0, 48)
            return s

        def setStyleSheet(self, s):
            self._doc.setDefaultStyleSheet(s)
            self._item_size = None

    def __init__(self, parent):
        super(HtmlListWidget, self).__init__(parent)
        bg = _BG_COLOR_DARK if _is_dark_mode() else _BG_COLOR_LIGHT
        QListWidget.setStyleSheet(self, "QListWidget{{background-color: {0};}}".format(bg))
        self._item_delegate = HtmlListWidget.HtmlItemDelegate(parent)
        self.setItemDelegate(self._item_delegate)

    def keyPressEvent(self, event):
        event.ignore()

    def setStyleSheet(self, s):
        self._item_delegate.setStyleSheet(s)


class WebView(QWebEngineView):
    wheelWithCtrl = Signal(int)

    def __init__(self, parent):
        super(WebView, self).__init__(parent)

        bg = _BG_COLOR_DARK if _is_dark_mode() else _BG_COLOR_LIGHT
        self.setStyleSheet("QWebEngineView{{background-color: {0};}}".format(bg))
        self.page().setBackgroundColor(QColor(bg))

        self._actionSearchText = QAction(self)
        if sys.platform != "darwin":
            self._actionSearchText.setIcon(
                QIcon.fromTheme("edit-find", QIcon(":/icons/edit-find.png"))
            )
        self._actionCopyPlain = QAction(self)
        self._actionCopyPlain.setText("Copy")
        if sys.platform != "darwin":
            self._actionCopyPlain.setIcon(
                QIcon.fromTheme("edit-copy", QIcon(":/icons/edit-copy.png"))
            )
        self._actionCopyPlain.triggered.connect(self._copyAsPlainText)
        self._actionCopyPlain.setShortcut(QKeySequence.StandardKey.Copy)
        self.page().selectionChanged.connect(self.__onSelectionChanged)
        self.__onSelectionChanged()
        self._actionDownloadAudio = QAction("Download mp3", self)

    def _copyAsPlainText(self):
        text = self.selectedText().strip()
        QApplication.clipboard().setText(text)

    @property
    def actionSearchText(self):
        return self._actionSearchText

    @property
    def actionCopyPlain(self):
        return self._actionCopyPlain

    @property
    def actionDownloadAudio(self):
        return self._actionDownloadAudio

    @property
    def audioUrlToDownload(self):
        return self._audioUrlToDownload

    def __onSelectionChanged(self):
        text = self.selectedText()
        self._actionCopyPlain.setEnabled(bool(text))

    def contextMenuEvent(self, event):
        # Remember the position where the context menu was invoked so actions
        # (like Anki export) can reference the correct DOM element later.
        try:
            self._last_context_pos = event.globalPos()
        except Exception:
            self._last_context_pos = None

        page = self.page()
        menu = self.createStandardContextMenu()
        actions = menu.actions()

        # Try WebKit-style APIs first (older versions); if not available, fall back
        # to QWebEngine's APIs + a small JavaScript snippet to extract surrounding HTML.
        header = ""
        meaning = ""
        try:
            # Old API (may not exist on QWebEnginePage)
            frame = page.frameAt(event.pos())
            hit_test_result = frame.hitTestContent(event.pos())

            header = frame.findFirstElement(".head").toOuterXml()
            header = header.replace("\n", "")

            meaning = hit_test_result.enclosingBlockElement().toOuterXml()
            meaning = meaning.replace("\n", "")
        except Exception:
            # Fallback for QWebEngine: try hitTestContent and run JavaScript to
            # find the closest .head and enclosing block element at the event point.
            try:
                # Call hitTestContent for its side effects if any, but we don't
                # need the returned object here.
                page.hitTestContent(event.pos())
            except Exception:
                pass

            # Coordinates: event.pos() is widget coordinates. For many pages this
            # maps well to document.elementFromPoint, but devicePixelRatio can
            # affect accuracy. Use the raw widget coords as a best-effort.
            p = event.pos()
            x = int(p.x())
            y = int(p.y())

            js = (
                "(function(){"
                "var ratio = window.devicePixelRatio || 1;"
                "var el = document.elementFromPoint(%d/ratio, %d/ratio);"
                "if(!el) return ['', ''];"
                "var head = el.closest('.head');"
                "var header = head ? head.outerHTML : '';"
                "var block = el.closest('p,div,li,section,article') || el;"
                "var meaning = block ? block.outerHTML : '';"
                "return [header, meaning];"
                "})()"
            ) % (x, y)

            # Run JS and wait briefly for result (runJavaScript is async)
            result = {}

            def _js_cb(res):
                result["value"] = res

            try:
                page.runJavaScript(js, _js_cb)
                # process events until callback sets result or timeout
                from time import time

                start = time()
                timeout = 1.0
                from PySide6.QtWidgets import QApplication

                while "value" not in result and time() - start < timeout:
                    QApplication.processEvents()

                if "value" in result and result["value"]:
                    header, meaning = result["value"]
                    if header is None:
                        header = ""
                    if meaning is None:
                        meaning = ""
            except Exception:
                # last-resort: use selectedText or empty strings
                try:
                    meaning = page.selectedText()
                except Exception:
                    meaning = ""

            # inserts the "Download audio" action
        # FIXME: when possible implement similar fallback JS extraction for link at pos
        # try:
        #     frame = page.frameAt(event.pos())
        #     hit_test_result = frame.hitTestContent(event.pos())
        #     if hit_test_result.linkUrl().scheme() == "audio":
        #         self._audioUrlToDownload = hit_test_result.linkUrl()
        #         menu.insertAction(actions[0] if actions else None, self.actionDownloadAudio)
        # except Exception:
        #     pass

        # Insert the "Search for ..." action
        text = page.selectedText().strip().lower()
        if text:
            text = ellipsis(text, 18)
            self._actionSearchText.setText('Lookup "{0}"'.format(text))
            menu.insertAction(actions[0] if actions else None, self.actionSearchText)

        # Replace WebKit's copy action with plain-text copying
        try:
            action_copy = page.action(QWebEnginePage.WebAction.Copy)
            if action_copy in actions:
                menu.insertAction(action_copy, self.actionCopyPlain)
                menu.removeAction(action_copy)
        except Exception:
            pass

        # display the context menu
        menu.exec_(event.globalPos())

    def keyPressEvent(self, event):
        if event.matches(QKeySequence.StandardKey.Copy):
            pass
        else:
            super(WebView, self).keyPressEvent(event)

    # --------------
    # Mouse Events
    # --------------

    def mousePressEvent(self, event):
        if sys.platform not in ("win32", "darwin"):
            if self.handleNavMouseButtons(event):
                return
        super(WebView, self).mousePressEvent(event)

    def mouseReleaseEvent(self, event):
        if sys.platform in ("win32", "darwin"):
            if self.handleNavMouseButtons(event):
                return
        super(WebView, self).mouseReleaseEvent(event)

    def wheelEvent(self, event):
        if event.modifiers() & Qt.KeyboardModifier.ControlModifier:
            self.wheelWithCtrl.emit(event.angleDelta().y())
            return
        super(WebView, self).wheelEvent(event)

    def handleNavMouseButtons(self, event):
        if event.button() == Qt.MouseButton.XButton1:
            self.triggerPageAction(QWebEnginePage.WebAction.Back)
            return True
        elif event.button() == Qt.MouseButton.XButton2:
            self.triggerPageAction(QWebEnginePage.WebAction.Forward)
            return True
        return False
