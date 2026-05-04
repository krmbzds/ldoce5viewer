# -*- coding: utf-8 -*-

################################################################################
## Form generated from reading UI file 'advanced.ui'
##
## Created by: Qt User Interface Compiler version 6.11.0
##
## WARNING! All changes made in this file will be lost when recompiling UI file!
################################################################################

from PySide6.QtCore import (QCoreApplication, QDate, QDateTime, QLocale,
    QMetaObject, QObject, QPoint, QRect,
    QSize, QTime, QUrl, Qt)
from PySide6.QtGui import (QBrush, QColor, QConicalGradient, QCursor,
    QFont, QFontDatabase, QGradient, QIcon,
    QImage, QKeySequence, QLinearGradient, QPainter,
    QPalette, QPixmap, QRadialGradient, QTransform)
from PySide6.QtWidgets import (QApplication, QDialog, QHBoxLayout, QHeaderView,
    QPushButton, QSizePolicy, QTreeWidget, QTreeWidgetItem,
    QVBoxLayout, QWidget)

from .custom import LineEdit

class Ui_Dialog(object):
    def setupUi(self, Dialog):
        if not Dialog.objectName():
            Dialog.setObjectName(u"Dialog")
        Dialog.resize(361, 468)
        self.verticalLayout = QVBoxLayout(Dialog)
        self.verticalLayout.setSpacing(5)
        self.verticalLayout.setContentsMargins(5, 5, 5, 5)
        self.verticalLayout.setObjectName(u"verticalLayout")
        self.horizontalLayout = QHBoxLayout()
        self.horizontalLayout.setSpacing(5)
#ifndef Q_OS_MAC
        self.horizontalLayout.setContentsMargins(0, 0, 0, 0)
#endif
        self.horizontalLayout.setObjectName(u"horizontalLayout")
        self.lineEditPhrase = LineEdit(Dialog)
        self.lineEditPhrase.setObjectName(u"lineEditPhrase")
        self.lineEditPhrase.setInputMethodHints(Qt.ImhDigitsOnly|Qt.ImhLowercaseOnly|Qt.ImhUppercaseOnly)

        self.horizontalLayout.addWidget(self.lineEditPhrase)

        self.buttonSearch = QPushButton(Dialog)
        self.buttonSearch.setObjectName(u"buttonSearch")
        sizePolicy = QSizePolicy(QSizePolicy.Policy.Minimum, QSizePolicy.Policy.Preferred)
        sizePolicy.setHorizontalStretch(0)
        sizePolicy.setVerticalStretch(0)
        sizePolicy.setHeightForWidth(self.buttonSearch.sizePolicy().hasHeightForWidth())
        self.buttonSearch.setSizePolicy(sizePolicy)
        icon = QIcon()
        iconThemeName = u"find"
        if QIcon.hasThemeIcon(iconThemeName):
            icon = QIcon.fromTheme(iconThemeName)
        else:
            icon.addFile(u"", QSize(), QIcon.Mode.Normal, QIcon.State.Off)

        self.buttonSearch.setIcon(icon)
        self.buttonSearch.setIconSize(QSize(16, 16))

        self.horizontalLayout.addWidget(self.buttonSearch)


        self.verticalLayout.addLayout(self.horizontalLayout)

        self.treeWidget = QTreeWidget(Dialog)
        __qtreewidgetitem = QTreeWidgetItem()
        __qtreewidgetitem.setText(0, u"1")
        self.treeWidget.setHeaderItem(__qtreewidgetitem)
        self.treeWidget.setObjectName(u"treeWidget")
        self.treeWidget.setHeaderHidden(True)

        self.verticalLayout.addWidget(self.treeWidget)

        self.buttonReset = QPushButton(Dialog)
        self.buttonReset.setObjectName(u"buttonReset")
        self.buttonReset.setIconSize(QSize(16, 16))

        self.verticalLayout.addWidget(self.buttonReset)

        QWidget.setTabOrder(self.treeWidget, self.buttonSearch)
        QWidget.setTabOrder(self.buttonSearch, self.lineEditPhrase)

        self.retranslateUi(Dialog)

        QMetaObject.connectSlotsByName(Dialog)
    # setupUi

    def retranslateUi(self, Dialog):
        Dialog.setWindowTitle(QCoreApplication.translate("Dialog", u"Advanced Search", None))
        self.lineEditPhrase.setPlaceholderText(QCoreApplication.translate("Dialog", u"Search phrase (optional)", None))
        self.buttonSearch.setText(QCoreApplication.translate("Dialog", u"Search", None))
        self.buttonReset.setText(QCoreApplication.translate("Dialog", u"Reset", None))
    # retranslateUi

