// @generated automatically by Diesel CLI.

diesel::table! {
    dict_words (id) {
        id -> Integer,
        dictionary_id -> Integer,
        dict_label -> Text,
        uid -> Text,
        word -> Text,
        word_ascii -> Text,
        language -> Nullable<Text>,
        source_uid -> Nullable<Text>,
        word_nom_sg -> Nullable<Text>,
        inflections -> Nullable<Text>,
        phonetic -> Nullable<Text>,
        transliteration -> Nullable<Text>,
        meaning_order -> Nullable<Integer>,
        definition_plain -> Nullable<Text>,
        definition_html -> Nullable<Text>,
        summary -> Nullable<Text>,
        synonyms -> Nullable<Text>,
        antonyms -> Nullable<Text>,
        homonyms -> Nullable<Text>,
        also_written_as -> Nullable<Text>,
        see_also -> Nullable<Text>,
        // created_at -> Nullable<Timestamp>,
        // updated_at -> Nullable<Timestamp>,
        // indexed_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    dictionaries (id) {
        id -> Integer,
        label -> Text,
        title -> Text,
        dict_type -> Text,
        creator -> Nullable<Text>,
        description -> Nullable<Text>,
        feedback_email -> Nullable<Text>,
        feedback_url -> Nullable<Text>,
        version -> Nullable<Text>,
        // created_at -> Nullable<Timestamp>,
        // updated_at -> Nullable<Timestamp>,
    }
}

diesel::joinable!(dict_words -> dictionaries (dictionary_id));

diesel::allow_tables_to_appear_in_same_query!(
    dict_words,
    dictionaries,
);
