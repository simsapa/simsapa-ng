use cxx_qt_lib::QString;

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("clipboard_manager.h");
        fn copy_with_mime_type_impl(text: &QString, mime_type: &QString);
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[namespace = "clipboard_manager"]
        type ClipboardManager = super::ClipboardManagerRust;

        #[qinvokable]
        #[cxx_name = "copyWithMimeType"]
        fn copy_with_mime_type(self: &ClipboardManager, text: &QString, mime_type: &QString);
    }
}

pub struct ClipboardManagerRust {}

impl Default for ClipboardManagerRust {
    fn default() -> Self {
        Self {}
    }
}

impl qobject::ClipboardManager {
    pub fn copy_with_mime_type(&self, text: &QString, mime_type: &QString) {
        qobject::copy_with_mime_type_impl(text, mime_type);
    }
}
