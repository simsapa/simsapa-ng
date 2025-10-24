#include "clipboard_manager.h"
#include <QGuiApplication>
#include <QClipboard>
#include <QMimeData>

void copy_with_mime_type_impl(const QString &text, const QString &mimeType)
{
    QClipboard *clipboard = QGuiApplication::clipboard();
    QMimeData *mimeData = new QMimeData();
    
    if (mimeType == "text/html") {
        mimeData->setHtml(text);
        mimeData->setText(text);
    } else if (mimeType == "text/plain") {
        mimeData->setText(text);
    } else if (mimeType == "text/markdown") {
        mimeData->setData("text/markdown", text.toUtf8());
        mimeData->setText(text);
    } else {
        mimeData->setData(mimeType, text.toUtf8());
        mimeData->setText(text);
    }
    
    clipboard->setMimeData(mimeData);
}
