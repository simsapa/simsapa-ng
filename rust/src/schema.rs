// @generated automatically by Diesel CLI.

diesel::table! {
    suttas (id) {
        id -> Integer,
        uid -> Text,
        sutta_ref -> Text,
        title -> Text,
        content_html -> Text,
    }
}
