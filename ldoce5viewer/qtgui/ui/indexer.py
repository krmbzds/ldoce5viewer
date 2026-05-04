# -*- coding: utf-8 -*-

################################################################################
## Form generated from reading UI file 'indexer.ui'
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
from PySide6.QtWidgets import (QApplication, QDialog, QHBoxLayout, QLabel,
    QLineEdit, QPlainTextEdit, QPushButton, QSizePolicy,
    QSpacerItem, QVBoxLayout, QWidget)

class Ui_Dialog(object):
    def setupUi(self, Dialog):
        if not Dialog.objectName():
            Dialog.setObjectName(u"Dialog")
        Dialog.resize(473, 395)
        self.verticalLayout = QVBoxLayout(Dialog)
        self.verticalLayout.setObjectName(u"verticalLayout")
        self.horizontalLayout1 = QHBoxLayout()
        self.horizontalLayout1.setObjectName(u"horizontalLayout1")
        self.label = QLabel(Dialog)
        self.label.setObjectName(u"label")

        self.horizontalLayout1.addWidget(self.label)

        self.lineEditPath = QLineEdit(Dialog)
        self.lineEditPath.setObjectName(u"lineEditPath")
        self.lineEditPath.setReadOnly(True)

        self.horizontalLayout1.addWidget(self.lineEditPath)

        self.buttonBrowseSource = QPushButton(Dialog)
        self.buttonBrowseSource.setObjectName(u"buttonBrowseSource")
        self.buttonBrowseSource.setIconSize(QSize(16, 16))

        self.horizontalLayout1.addWidget(self.buttonBrowseSource)


        self.verticalLayout.addLayout(self.horizontalLayout1)

        self.plainTextEdit = QPlainTextEdit(Dialog)
        self.plainTextEdit.setObjectName(u"plainTextEdit")
        self.plainTextEdit.setEnabled(True)
        self.plainTextEdit.setUndoRedoEnabled(False)
        self.plainTextEdit.setReadOnly(True)
        self.plainTextEdit.setTextInteractionFlags(Qt.TextBrowserInteraction)
        self.plainTextEdit.setBackgroundVisible(False)

        self.verticalLayout.addWidget(self.plainTextEdit)

        self.horizontalLayout2 = QHBoxLayout()
        self.horizontalLayout2.setObjectName(u"horizontalLayout2")
        self.horizontalSpacer = QSpacerItem(40, 20, QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Minimum)

        self.horizontalLayout2.addItem(self.horizontalSpacer)

        self.buttonCancel = QPushButton(Dialog)
        self.buttonCancel.setObjectName(u"buttonCancel")

        self.horizontalLayout2.addWidget(self.buttonCancel)

        self.buttonRun = QPushButton(Dialog)
        self.buttonRun.setObjectName(u"buttonRun")

        self.horizontalLayout2.addWidget(self.buttonRun)


        self.verticalLayout.addLayout(self.horizontalLayout2)


        self.retranslateUi(Dialog)

        QMetaObject.connectSlotsByName(Dialog)
    # setupUi

    def retranslateUi(self, Dialog):
        Dialog.setWindowTitle(QCoreApplication.translate("Dialog", u"Create Index", None))
        self.label.setText(QCoreApplication.translate("Dialog", u"Data Location:", None))
        self.buttonBrowseSource.setText(QCoreApplication.translate("Dialog", u"Browse...", None))
        self.buttonCancel.setText(QCoreApplication.translate("Dialog", u"Cancel", None))
        self.buttonRun.setText(QCoreApplication.translate("Dialog", u"Start Indexing", None))
    # retranslateUi

