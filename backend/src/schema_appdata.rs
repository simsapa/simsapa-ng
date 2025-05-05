// @generated automatically by Diesel CLI.

diesel::table! {
    app_settings (id) {
        id -> Integer,
        key -> Text,
        value -> Nullable<Text>,
        // created_at -> Nullable<Timestamp>,
        // updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    suttas (id) {
        id -> Integer,
        uid -> Text,
        sutta_ref -> Text,
        nikaya -> Text,
        language -> Text,
        group_path -> Nullable<Text>,
        group_index -> Nullable<Integer>,
        order_index -> Nullable<Integer>,
        sutta_range_group -> Nullable<Text>,
        sutta_range_start -> Nullable<Integer>,
        sutta_range_end -> Nullable<Integer>,
        title -> Nullable<Text>,
        title_ascii -> Nullable<Text>,
        title_pali -> Nullable<Text>,
        title_trans -> Nullable<Text>,
        description -> Nullable<Text>,
        content_plain -> Nullable<Text>,
        content_html -> Nullable<Text>,
        content_json -> Nullable<Text>,
        content_json_tmpl -> Nullable<Text>,
        source_uid -> Nullable<Text>,
        source_info -> Nullable<Text>,
        source_language -> Nullable<Text>,
        message -> Nullable<Text>,
        copyright -> Nullable<Text>,
        license -> Nullable<Text>,
        // created_at -> Nullable<Timestamp>,
        // updated_at -> Nullable<Timestamp>,
        // indexed_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    sutta_variants (id) {
        id -> Integer,
        sutta_id -> Integer,
        sutta_uid -> Text,
        language -> Nullable<Text>,
        source_uid -> Nullable<Text>,
        content_json -> Nullable<Text>,
        // created_at -> Nullable<Timestamp>,
        // updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    sutta_comments (id) {
        id -> Integer,
        sutta_id -> Integer,
        sutta_uid -> Text,
        language -> Nullable<Text>,
        source_uid -> Nullable<Text>,
        content_json -> Nullable<Text>,
        // created_at -> Nullable<Timestamp>,
        // updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    sutta_glosses (id) {
        id -> Integer,
        sutta_id -> Integer,
        sutta_uid -> Text,
        language -> Nullable<Text>,
        source_uid -> Nullable<Text>,
        content_json -> Nullable<Text>,
        // created_at -> Nullable<Timestamp>,
        // updated_at -> Nullable<Timestamp>,
    }
}

diesel::joinable!(sutta_variants -> suttas (sutta_id));
diesel::joinable!(sutta_comments -> suttas (sutta_id));
diesel::joinable!(sutta_glosses -> suttas (sutta_id));

diesel::allow_tables_to_appear_in_same_query!(
    app_settings,
    suttas,
    sutta_variants,
    sutta_comments,
    sutta_glosses,
);
