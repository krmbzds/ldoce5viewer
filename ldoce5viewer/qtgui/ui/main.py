# -*- coding: utf-8 -*-

################################################################################
## Form generated from reading UI file 'main.ui'
##
## Created by: Qt User Interface Compiler version 6.11.0
##
## WARNING! All changes made in this file will be lost when recompiling UI file!
################################################################################

from PySide6.QtCore import (QCoreApplication, QDate, QDateTime, QLocale,
    QMetaObject, QObject, QPoint, QRect,
    QSize, QTime, QUrl, Qt)
from PySide6.QtGui import (QAction, QBrush, QColor, QConicalGradient,
    QCursor, QFont, QFontDatabase, QGradient,
    QIcon, QImage, QKeySequence, QLinearGradient,
    QPainter, QPalette, QPixmap, QRadialGradient,
    QTransform)
from PySide6.QtWidgets import (QApplication, QFrame, QHBoxLayout, QLabel,
    QListWidgetItem, QMainWindow, QMenu, QMenuBar,
    QSizePolicy, QSpacerItem, QSplitter, QToolBar,
    QToolButton, QVBoxLayout, QWidget)

from .custom import (HtmlListWidget, LineEditFind, WebView)

class Ui_MainWindow(object):
    def setupUi(self, MainWindow):
        if not MainWindow.objectName():
            MainWindow.setObjectName(u"MainWindow")
        MainWindow.resize(800, 700)
        MainWindow.setUnifiedTitleAndToolBarOnMac(True)
        self.actionQuit = QAction(MainWindow)
        self.actionQuit.setObjectName(u"actionQuit")
        icon = QIcon()
        iconThemeName = u"application-exit"
        if QIcon.hasThemeIcon(iconThemeName):
            icon = QIcon.fromTheme(iconThemeName)
        else:
            icon.addFile(u"", QSize(), QIcon.Mode.Normal, QIcon.State.Off)

        self.actionQuit.setIcon(icon)
        self.actionZoomIn = QAction(MainWindow)
        self.actionZoomIn.setObjectName(u"actionZoomIn")
        icon1 = QIcon()
        iconThemeName = u"zoom-in"
        if QIcon.hasThemeIcon(iconThemeName):
            icon1 = QIcon.fromTheme(iconThemeName)
        else:
            icon1.addFile(u"", QSize(), QIcon.Mode.Normal, QIcon.State.Off)

        self.actionZoomIn.setIcon(icon1)
        self.actionZoomOut = QAction(MainWindow)
        self.actionZoomOut.setObjectName(u"actionZoomOut")
        icon2 = QIcon()
        iconThemeName = u"zoom-out"
        if QIcon.hasThemeIcon(iconThemeName):
            icon2 = QIcon.fromTheme(iconThemeName)
        else:
            icon2.addFile(u"", QSize(), QIcon.Mode.Normal, QIcon.State.Off)

        self.actionZoomOut.setIcon(icon2)
        self.actionNormalSize = QAction(MainWindow)
        self.actionNormalSize.setObjectName(u"actionNormalSize")
        icon3 = QIcon()
        iconThemeName = u"zoom-original"
        if QIcon.hasThemeIcon(iconThemeName):
            icon3 = QIcon.fromTheme(iconThemeName)
        else:
            icon3.addFile(u"", QSize(), QIcon.Mode.Normal, QIcon.State.Off)

        self.actionNormalSize.setIcon(icon3)
        self.actionFind = QAction(MainWindow)
        self.actionFind.setObjectName(u"actionFind")
        icon4 = QIcon()
        iconThemeName = u"edit-find"
        if QIcon.hasThemeIcon(iconThemeName):
            icon4 = QIcon.fromTheme(iconThemeName)
        else:
            icon4.addFile(u"", QSize(), QIcon.Mode.Normal, QIcon.State.Off)

        self.actionFind.setIcon(icon4)
        self.actionFindPrev = QAction(MainWindow)
        self.actionFindPrev.setObjectName(u"actionFindPrev")
        icon5 = QIcon()
        iconThemeName = u"go-up"
        if QIcon.hasThemeIcon(iconThemeName):
            icon5 = QIcon.fromTheme(iconThemeName)
        else:
            icon5.addFile(u"", QSize(), QIcon.Mode.Normal, QIcon.State.Off)

        self.actionFindPrev.setIcon(icon5)
        self.actionFindNext = QAction(MainWindow)
        self.actionFindNext.setObjectName(u"actionFindNext")
        icon6 = QIcon()
        iconThemeName = u"go-down"
        if QIcon.hasThemeIcon(iconThemeName):
            icon6 = QIcon.fromTheme(iconThemeName)
        else:
            icon6.addFile(u"", QSize(), QIcon.Mode.Normal, QIcon.State.Off)

        self.actionFindNext.setIcon(icon6)
        self.actionFindClose = QAction(MainWindow)
        self.actionFindClose.setObjectName(u"actionFindClose")
        icon7 = QIcon()
        iconThemeName = u"window-close"
        if QIcon.hasThemeIcon(iconThemeName):
            icon7 = QIcon.fromTheme(iconThemeName)
        else:
            icon7.addFile(u"", QSize(), QIcon.Mode.Normal, QIcon.State.Off)

        self.actionFindClose.setIcon(icon7)
        self.actionAbout = QAction(MainWindow)
        self.actionAbout.setObjectName(u"actionAbout")
        icon8 = QIcon()
        iconThemeName = u"help-about"
        if QIcon.hasThemeIcon(iconThemeName):
            icon8 = QIcon.fromTheme(iconThemeName)
        else:
            icon8.addFile(u"", QSize(), QIcon.Mode.Normal, QIcon.State.Off)

        self.actionAbout.setIcon(icon8)
        self.actionPrint = QAction(MainWindow)
        self.actionPrint.setObjectName(u"actionPrint")
        icon9 = QIcon()
        iconThemeName = u"document-print"
        if QIcon.hasThemeIcon(iconThemeName):
            icon9 = QIcon.fromTheme(iconThemeName)
        else:
            icon9.addFile(u"", QSize(), QIcon.Mode.Normal, QIcon.State.Off)

        self.actionPrint.setIcon(icon9)
        self.actionPrintPreview = QAction(MainWindow)
        self.actionPrintPreview.setObjectName(u"actionPrintPreview")
        icon10 = QIcon()
        iconThemeName = u"document-print-preview"
        if QIcon.hasThemeIcon(iconThemeName):
            icon10 = QIcon.fromTheme(iconThemeName)
        else:
            icon10.addFile(u"", QSize(), QIcon.Mode.Normal, QIcon.State.Off)

        self.actionPrintPreview.setIcon(icon10)
        self.actionCreateIndex = QAction(MainWindow)
        self.actionCreateIndex.setObjectName(u"actionCreateIndex")
        icon11 = QIcon()
        iconThemeName = u"document-properties"
        if QIcon.hasThemeIcon(iconThemeName):
            icon11 = QIcon.fromTheme(iconThemeName)
        else:
            icon11.addFile(u"", QSize(), QIcon.Mode.Normal, QIcon.State.Off)

        self.actionCreateIndex.setIcon(icon11)
        self.actionCloseInspector = QAction(MainWindow)
        self.actionCloseInspector.setObjectName(u"actionCloseInspector")
        self.actionCloseInspector.setIcon(icon7)
        self.actionMonitorClipboard = QAction(MainWindow)
        self.actionMonitorClipboard.setObjectName(u"actionMonitorClipboard")
        self.actionMonitorClipboard.setCheckable(True)
        self.actionSearchExamples = QAction(MainWindow)
        self.actionSearchExamples.setObjectName(u"actionSearchExamples")
        icon12 = QIcon()
        icon12.addFile(u"", QSize(), QIcon.Mode.Normal, QIcon.State.Off)
        self.actionSearchExamples.setIcon(icon12)
        self.actionPronUS = QAction(MainWindow)
        self.actionPronUS.setObjectName(u"actionPronUS")
        self.actionPronUS.setCheckable(True)
        self.actionPronGB = QAction(MainWindow)
        self.actionPronGB.setObjectName(u"actionPronGB")
        self.actionPronGB.setCheckable(True)
        self.actionPronOff = QAction(MainWindow)
        self.actionPronOff.setObjectName(u"actionPronOff")
        self.actionPronOff.setCheckable(True)
        self.actionAdvancedSearch = QAction(MainWindow)
        self.actionAdvancedSearch.setObjectName(u"actionAdvancedSearch")
        self.actionAdvancedSearch.setIcon(icon12)
        self.actionFocusLineEdit = QAction(MainWindow)
        self.actionFocusLineEdit.setObjectName(u"actionFocusLineEdit")
        self.actionHelp = QAction(MainWindow)
        self.actionHelp.setObjectName(u"actionHelp")
        icon13 = QIcon()
        iconThemeName = u"help-contents"
        if QIcon.hasThemeIcon(iconThemeName):
            icon13 = QIcon.fromTheme(iconThemeName)
        else:
            icon13.addFile(u"", QSize(), QIcon.Mode.Normal, QIcon.State.Off)

        self.actionHelp.setIcon(icon13)
        self.actionAlwaysOnTop = QAction(MainWindow)
        self.actionAlwaysOnTop.setObjectName(u"actionAlwaysOnTop")
        self.actionAlwaysOnTop.setCheckable(True)
        self.actionSearchDefinitions = QAction(MainWindow)
        self.actionSearchDefinitions.setObjectName(u"actionSearchDefinitions")
        self.actionSearchDefinitions.setIcon(icon12)
        self.centralwidget = QWidget(MainWindow)
        self.centralwidget.setObjectName(u"centralwidget")
        self.verticalLayout = QVBoxLayout(self.centralwidget)
        self.verticalLayout.setSpacing(0)
        self.verticalLayout.setObjectName(u"verticalLayout")
        self.verticalLayout.setContentsMargins(0, 0, 0, 0)
        self.splitter = QSplitter(self.centralwidget)
        self.splitter.setObjectName(u"splitter")
        self.splitter.setOrientation(Qt.Horizontal)
        self.leftPane = QWidget(self.splitter)
        self.leftPane.setObjectName(u"leftPane")
        self.verticalLayoutLeft = QVBoxLayout(self.leftPane)
        self.verticalLayoutLeft.setSpacing(2)
        self.verticalLayoutLeft.setObjectName(u"verticalLayoutLeft")
        self.verticalLayoutLeft.setContentsMargins(0, 0, 0, 0)
        self.labelSearching = QLabel(self.leftPane)
        self.labelSearching.setObjectName(u"labelSearching")
        self.labelSearching.setFrameShape(QFrame.NoFrame)
        self.labelSearching.setAlignment(Qt.AlignCenter)

        self.verticalLayoutLeft.addWidget(self.labelSearching)

        self.listWidgetIndex = HtmlListWidget(self.leftPane)
        self.listWidgetIndex.setObjectName(u"listWidgetIndex")
        self.listWidgetIndex.setEnabled(True)
        sizePolicy = QSizePolicy(QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Expanding)
        sizePolicy.setHorizontalStretch(0)
        sizePolicy.setVerticalStretch(0)
        sizePolicy.setHeightForWidth(self.listWidgetIndex.sizePolicy().hasHeightForWidth())
        self.listWidgetIndex.setSizePolicy(sizePolicy)
        self.listWidgetIndex.setFrameShape(QFrame.NoFrame)

        self.verticalLayoutLeft.addWidget(self.listWidgetIndex)

        self.splitter.addWidget(self.leftPane)
        self.mainPane = QFrame(self.splitter)
        self.mainPane.setObjectName(u"mainPane")
        sizePolicy1 = QSizePolicy(QSizePolicy.Policy.Preferred, QSizePolicy.Policy.Preferred)
        sizePolicy1.setHorizontalStretch(2)
        sizePolicy1.setVerticalStretch(0)
        sizePolicy1.setHeightForWidth(self.mainPane.sizePolicy().hasHeightForWidth())
        self.mainPane.setSizePolicy(sizePolicy1)
        self.verticalLayout_4 = QVBoxLayout(self.mainPane)
        self.verticalLayout_4.setSpacing(0)
        self.verticalLayout_4.setObjectName(u"verticalLayout_4")
        self.verticalLayout_4.setContentsMargins(0, 0, 0, 0)
        self.frameFindbar = QFrame(self.mainPane)
        self.frameFindbar.setObjectName(u"frameFindbar")
        sizePolicy2 = QSizePolicy(QSizePolicy.Policy.Minimum, QSizePolicy.Policy.Minimum)
        sizePolicy2.setHorizontalStretch(0)
        sizePolicy2.setVerticalStretch(0)
        sizePolicy2.setHeightForWidth(self.frameFindbar.sizePolicy().hasHeightForWidth())
        self.frameFindbar.setSizePolicy(sizePolicy2)
        self.horizontalLayout_2 = QHBoxLayout(self.frameFindbar)
        self.horizontalLayout_2.setSpacing(5)
        self.horizontalLayout_2.setObjectName(u"horizontalLayout_2")
        self.horizontalLayout_2.setContentsMargins(2, 6, 0, 6)
        self.toolButtonCloseFindbar = QToolButton(self.frameFindbar)
        self.toolButtonCloseFindbar.setObjectName(u"toolButtonCloseFindbar")
        self.toolButtonCloseFindbar.setIcon(icon7)
        self.toolButtonCloseFindbar.setIconSize(QSize(16, 16))
        self.toolButtonCloseFindbar.setAutoRaise(True)

        self.horizontalLayout_2.addWidget(self.toolButtonCloseFindbar)

        self.labelFind = QLabel(self.frameFindbar)
        self.labelFind.setObjectName(u"labelFind")
        sizePolicy3 = QSizePolicy(QSizePolicy.Policy.Preferred, QSizePolicy.Policy.Preferred)
        sizePolicy3.setHorizontalStretch(0)
        sizePolicy3.setVerticalStretch(0)
        sizePolicy3.setHeightForWidth(self.labelFind.sizePolicy().hasHeightForWidth())
        self.labelFind.setSizePolicy(sizePolicy3)

        self.horizontalLayout_2.addWidget(self.labelFind)

        self.lineEditFind = LineEditFind(self.frameFindbar)
        self.lineEditFind.setObjectName(u"lineEditFind")
        sizePolicy4 = QSizePolicy(QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Fixed)
        sizePolicy4.setHorizontalStretch(1)
        sizePolicy4.setVerticalStretch(0)
        sizePolicy4.setHeightForWidth(self.lineEditFind.sizePolicy().hasHeightForWidth())
        self.lineEditFind.setSizePolicy(sizePolicy4)
        self.lineEditFind.setMaximumSize(QSize(250, 16777215))
        self.lineEditFind.setLayoutDirection(Qt.LeftToRight)
        self.lineEditFind.setFrame(True)

        self.horizontalLayout_2.addWidget(self.lineEditFind)

        self.horizontalLayout = QHBoxLayout()
        self.horizontalLayout.setSpacing(3)
        self.horizontalLayout.setObjectName(u"horizontalLayout")
        self.horizontalLayout.setContentsMargins(0, 0, 0, 0)
        self.toolButtonFindNext = QToolButton(self.frameFindbar)
        self.toolButtonFindNext.setObjectName(u"toolButtonFindNext")
        self.toolButtonFindNext.setIcon(icon6)
        self.toolButtonFindNext.setIconSize(QSize(16, 16))
        self.toolButtonFindNext.setToolButtonStyle(Qt.ToolButtonTextBesideIcon)
        self.toolButtonFindNext.setAutoRaise(True)
        self.toolButtonFindNext.setArrowType(Qt.NoArrow)

        self.horizontalLayout.addWidget(self.toolButtonFindNext)

        self.toolButtonFindPrev = QToolButton(self.frameFindbar)
        self.toolButtonFindPrev.setObjectName(u"toolButtonFindPrev")
        self.toolButtonFindPrev.setIcon(icon5)
        self.toolButtonFindPrev.setIconSize(QSize(16, 16))
        self.toolButtonFindPrev.setToolButtonStyle(Qt.ToolButtonTextBesideIcon)
        self.toolButtonFindPrev.setAutoRaise(True)
        self.toolButtonFindPrev.setArrowType(Qt.NoArrow)

        self.horizontalLayout.addWidget(self.toolButtonFindPrev)

        self.labelFindResults = QLabel(self.frameFindbar)
        self.labelFindResults.setObjectName(u"labelFindResults")
        sizePolicy3.setHeightForWidth(self.labelFindResults.sizePolicy().hasHeightForWidth())
        self.labelFindResults.setSizePolicy(sizePolicy3)

        self.horizontalLayout.addWidget(self.labelFindResults)

        self.horizontalSpacer = QSpacerItem(0, 0, QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Minimum)

        self.horizontalLayout.addItem(self.horizontalSpacer)


        self.horizontalLayout_2.addLayout(self.horizontalLayout)


        self.verticalLayout_4.addWidget(self.frameFindbar)

        self.splitter2 = QSplitter(self.mainPane)
        self.splitter2.setObjectName(u"splitter2")
        self.splitter2.setOrientation(Qt.Vertical)
        self.frame = QFrame(self.splitter2)
        self.frame.setObjectName(u"frame")
        self.frame.setFrameShape(QFrame.NoFrame)
        self.frame.setLineWidth(1)
        self.frame.setMidLineWidth(1)
        self.verticalLayout_3 = QVBoxLayout(self.frame)
        self.verticalLayout_3.setSpacing(5)
        self.verticalLayout_3.setObjectName(u"verticalLayout_3")
        self.verticalLayout_3.setContentsMargins(0, 0, 0, 0)
        self.webView = WebView(self.frame)
        self.webView.setObjectName(u"webView")
        sizePolicy.setHeightForWidth(self.webView.sizePolicy().hasHeightForWidth())
        self.webView.setSizePolicy(sizePolicy)
        self.webView.setProperty(u"url", QUrl(u"about:blank"))

        self.verticalLayout_3.addWidget(self.webView)

        self.splitter2.addWidget(self.frame)
        self.inspectorContainer = QFrame(self.splitter2)
        self.inspectorContainer.setObjectName(u"inspectorContainer")
        self.inspectorContainer.setFrameShape(QFrame.NoFrame)
        self.verticalLayout_2 = QVBoxLayout(self.inspectorContainer)
        self.verticalLayout_2.setSpacing(5)
        self.verticalLayout_2.setObjectName(u"verticalLayout_2")
        self.verticalLayout_2.setContentsMargins(0, 0, 0, 0)
        self.horizontalLayout3 = QHBoxLayout()
        self.horizontalLayout3.setObjectName(u"horizontalLayout3")
        self.horizontalLayout3.setContentsMargins(0, 0, 2, 0)
        self.toolButtonCloseInspector = QToolButton(self.inspectorContainer)
        self.toolButtonCloseInspector.setObjectName(u"toolButtonCloseInspector")
        self.toolButtonCloseInspector.setIcon(icon7)
        self.toolButtonCloseInspector.setIconSize(QSize(16, 16))
        self.toolButtonCloseInspector.setAutoRaise(True)

        self.horizontalLayout3.addWidget(self.toolButtonCloseInspector)


        self.verticalLayout_2.addLayout(self.horizontalLayout3)

        self.webInspector = QWidget(self.inspectorContainer)
        self.webInspector.setObjectName(u"webInspector")
        sizePolicy.setHeightForWidth(self.webInspector.sizePolicy().hasHeightForWidth())
        self.webInspector.setSizePolicy(sizePolicy)

        self.verticalLayout_2.addWidget(self.webInspector)

        self.splitter2.addWidget(self.inspectorContainer)

        self.verticalLayout_4.addWidget(self.splitter2)

        self.splitter.addWidget(self.mainPane)
        self.splitter2.raise_()
        self.frameFindbar.raise_()

        self.verticalLayout.addWidget(self.splitter)

        MainWindow.setCentralWidget(self.centralwidget)
        self.menuBar = QMenuBar(MainWindow)
        self.menuBar.setObjectName(u"menuBar")
        self.menuBar.setGeometry(QRect(0, 0, 800, 19))
        self.menuViewer = QMenu(self.menuBar)
        self.menuViewer.setObjectName(u"menuViewer")
        self.menuOptions = QMenu(self.menuBar)
        self.menuOptions.setObjectName(u"menuOptions")
        self.menuEdit = QMenu(self.menuBar)
        self.menuEdit.setObjectName(u"menuEdit")
        self.menuHelp = QMenu(self.menuBar)
        self.menuHelp.setObjectName(u"menuHelp")
        MainWindow.setMenuBar(self.menuBar)
        self.toolBar = QToolBar(MainWindow)
        self.toolBar.setObjectName(u"toolBar")
        self.toolBar.setMovable(False)
        self.toolBar.setAllowedAreas(Qt.TopToolBarArea)
        self.toolBar.setIconSize(QSize(24, 24))
        self.toolBar.setToolButtonStyle(Qt.ToolButtonIconOnly)
        self.toolBar.setFloatable(False)
        MainWindow.addToolBar(Qt.ToolBarArea.TopToolBarArea, self.toolBar)
        QWidget.setTabOrder(self.toolButtonCloseFindbar, self.lineEditFind)

        self.menuBar.addAction(self.menuViewer.menuAction())
        self.menuBar.addAction(self.menuEdit.menuAction())
        self.menuBar.addAction(self.menuOptions.menuAction())
        self.menuBar.addAction(self.menuHelp.menuAction())
        self.menuViewer.addAction(self.actionPrintPreview)
        self.menuViewer.addAction(self.actionPrint)
        self.menuViewer.addSeparator()
        self.menuViewer.addAction(self.actionCreateIndex)
        self.menuViewer.addSeparator()
        self.menuViewer.addAction(self.actionQuit)
        self.menuOptions.addAction(self.actionZoomIn)
        self.menuOptions.addAction(self.actionZoomOut)
        self.menuOptions.addAction(self.actionNormalSize)
        self.menuOptions.addSeparator()
        self.menuOptions.addAction(self.actionMonitorClipboard)
        self.menuOptions.addSeparator()
        self.menuOptions.addAction(self.actionPronOff)
        self.menuOptions.addAction(self.actionPronGB)
        self.menuOptions.addAction(self.actionPronUS)
        self.menuEdit.addAction(self.actionFind)
        self.menuEdit.addSeparator()
        self.menuHelp.addAction(self.actionHelp)
        self.menuHelp.addAction(self.actionAbout)

        self.retranslateUi(MainWindow)
        self.actionQuit.triggered.connect(MainWindow.close)

        QMetaObject.connectSlotsByName(MainWindow)
    # setupUi

    def retranslateUi(self, MainWindow):
        self.actionQuit.setText(QCoreApplication.translate("MainWindow", u"Quit", None))
        self.actionZoomIn.setText(QCoreApplication.translate("MainWindow", u"Zoom In", None))
        self.actionZoomOut.setText(QCoreApplication.translate("MainWindow", u"Zoom Out", None))
        self.actionNormalSize.setText(QCoreApplication.translate("MainWindow", u"Normal Size", None))
        self.actionFind.setText(QCoreApplication.translate("MainWindow", u"Find...", None))
        self.actionFindPrev.setText(QCoreApplication.translate("MainWindow", u"Previous", None))
#if QT_CONFIG(tooltip)
        self.actionFindPrev.setToolTip(QCoreApplication.translate("MainWindow", u"Find the previous occurrence", None))
#endif // QT_CONFIG(tooltip)
        self.actionFindNext.setText(QCoreApplication.translate("MainWindow", u"Next", None))
#if QT_CONFIG(tooltip)
        self.actionFindNext.setToolTip(QCoreApplication.translate("MainWindow", u"Find the next occurrence", None))
#endif // QT_CONFIG(tooltip)
        self.actionFindClose.setText(QCoreApplication.translate("MainWindow", u"Close", None))
#if QT_CONFIG(tooltip)
        self.actionFindClose.setToolTip(QCoreApplication.translate("MainWindow", u"Close find bar", None))
#endif // QT_CONFIG(tooltip)
        self.actionAbout.setText(QCoreApplication.translate("MainWindow", u"About", None))
        self.actionPrint.setText(QCoreApplication.translate("MainWindow", u"Print...", None))
        self.actionPrintPreview.setText(QCoreApplication.translate("MainWindow", u"Print Preview", None))
        self.actionCreateIndex.setText(QCoreApplication.translate("MainWindow", u"Recreate Index...", None))
        self.actionCloseInspector.setText(QCoreApplication.translate("MainWindow", u"Close Inspector", None))
#if QT_CONFIG(tooltip)
        self.actionCloseInspector.setToolTip(QCoreApplication.translate("MainWindow", u"Close inspector", None))
#endif // QT_CONFIG(tooltip)
        self.actionMonitorClipboard.setText(QCoreApplication.translate("MainWindow", u"Monitor Clipboard", None))
        self.actionSearchExamples.setText(QCoreApplication.translate("MainWindow", u"Exa", None))
#if QT_CONFIG(tooltip)
        self.actionSearchExamples.setToolTip(QCoreApplication.translate("MainWindow", u"Example Search", None))
#endif // QT_CONFIG(tooltip)
        self.actionPronUS.setText(QCoreApplication.translate("MainWindow", u"American Pronunciation", None))
#if QT_CONFIG(shortcut)
        self.actionPronUS.setShortcut(QCoreApplication.translate("MainWindow", u"Ctrl+Shift+A", None))
#endif // QT_CONFIG(shortcut)
        self.actionPronGB.setText(QCoreApplication.translate("MainWindow", u"British Pronunciation", None))
#if QT_CONFIG(shortcut)
        self.actionPronGB.setShortcut(QCoreApplication.translate("MainWindow", u"Ctrl+Shift+B", None))
#endif // QT_CONFIG(shortcut)
        self.actionPronOff.setText(QCoreApplication.translate("MainWindow", u"No Sound", None))
#if QT_CONFIG(shortcut)
        self.actionPronOff.setShortcut(QCoreApplication.translate("MainWindow", u"Ctrl+Shift+N", None))
#endif // QT_CONFIG(shortcut)
        self.actionAdvancedSearch.setText(QCoreApplication.translate("MainWindow", u"Advanced", None))
#if QT_CONFIG(tooltip)
        self.actionAdvancedSearch.setToolTip(QCoreApplication.translate("MainWindow", u"Advanced Search", None))
#endif // QT_CONFIG(tooltip)
        self.actionFocusLineEdit.setText(QCoreApplication.translate("MainWindow", u"actionFocusLineEdit", None))
        self.actionHelp.setText(QCoreApplication.translate("MainWindow", u"Help", None))
#if QT_CONFIG(tooltip)
        self.actionHelp.setToolTip(QCoreApplication.translate("MainWindow", u"Help", None))
#endif // QT_CONFIG(tooltip)
        self.actionAlwaysOnTop.setText(QCoreApplication.translate("MainWindow", u"Always on Top", None))
        self.actionSearchDefinitions.setText(QCoreApplication.translate("MainWindow", u"Def", None))
#if QT_CONFIG(tooltip)
        self.actionSearchDefinitions.setToolTip(QCoreApplication.translate("MainWindow", u"Definition Search", None))
#endif // QT_CONFIG(tooltip)
        self.labelSearching.setText(QCoreApplication.translate("MainWindow", u"<html><head/><body><p>Searching...</p></body></html>", None))
        self.toolButtonCloseFindbar.setText(QCoreApplication.translate("MainWindow", u"Close", None))
        self.labelFind.setText(QCoreApplication.translate("MainWindow", u"Find:", None))
        self.lineEditFind.setPlaceholderText("")
#if QT_CONFIG(tooltip)
        self.toolButtonFindNext.setToolTip("")
#endif // QT_CONFIG(tooltip)
        self.toolButtonFindNext.setText(QCoreApplication.translate("MainWindow", u"Next", None))
        self.toolButtonFindPrev.setText(QCoreApplication.translate("MainWindow", u"Previous", None))
        self.labelFindResults.setText("")
        self.toolButtonCloseInspector.setText(QCoreApplication.translate("MainWindow", u"...", None))
        self.menuViewer.setTitle(QCoreApplication.translate("MainWindow", u"&Viewer", None))
        self.menuOptions.setTitle(QCoreApplication.translate("MainWindow", u"&Options", None))
        self.menuEdit.setTitle(QCoreApplication.translate("MainWindow", u"&Edit", None))
        self.menuHelp.setTitle(QCoreApplication.translate("MainWindow", u"&Help", None))
        self.toolBar.setWindowTitle(QCoreApplication.translate("MainWindow", u"toolBar", None))
        pass
    # retranslateUi

