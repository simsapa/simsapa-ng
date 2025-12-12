#ifndef CLIPBOARD_MANAGER_H_
#define CLIPBOARD_MANAGER_H_

#include <QString>

void copy_with_mime_type_impl(const QString &text, const QString &mimeType);
bool open_external_url_impl(const QString &url);

#endif // CLIPBOARD_MANAGER_H_
