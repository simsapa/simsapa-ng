// @generated automatically by Diesel CLI.

diesel::table! {
    bold_definitions (id) {
        id -> Integer,
        file_name -> Text,
        ref_code -> Text,
        nikaya -> Text,
        book -> Text,
        title -> Text,
        subhead -> Text,
        bold -> Text,
        bold_end -> Text,
        commentary -> Text,
    }
}

diesel::table! {
    db_info (id) {
        id -> Integer,
        key -> Text,
        value -> Text,
    }
}

diesel::table! {
    dpd_headwords (id) {
        id -> Integer,
        lemma_1 -> Text,
        lemma_2 -> Text,
        pos -> Text,
        grammar -> Text,
        derived_from -> Text,
        neg -> Text,
        verb -> Text,
        trans -> Text,
        plus_case -> Text,
        meaning_1 -> Text,
        meaning_lit -> Text,
        meaning_2 -> Text,
        non_ia -> Text,
        sanskrit -> Text,
        root_key -> Text,
        root_sign -> Text,
        root_base -> Text,
        family_root_fk -> Text, // renamed
        family_word_fk -> Text, // renamed
        family_compound_fk -> Text, // renamed
        family_idioms_fk -> Text, // renamed
        family_set_fk -> Text, // renamed
        construction -> Text,
        derivative -> Text,
        suffix -> Text,
        phonetic -> Text,
        compound_type -> Text,
        compound_construction -> Text,
        non_root_in_comps -> Text,
        source_1 -> Text,
        sutta_1 -> Text,
        example_1 -> Text,
        source_2 -> Text,
        sutta_2 -> Text,
        example_2 -> Text,
        antonym -> Text,
        synonym -> Text,
        variant -> Text,
        var_phonetic -> Text,
        var_text -> Text,
        commentary -> Text,
        notes -> Text,
        cognate -> Text,
        link -> Text,
        origin -> Text,
        stem -> Text,
        pattern -> Text,
        // created_at -> Nullable<Timestamp>,
        // updated_at -> Nullable<Timestamp>,
        inflections -> Text,
        inflections_api_ca_eva_iti -> Text,
        inflections_sinhala -> Text,
        inflections_devanagari -> Text,
        inflections_thai -> Text,
        inflections_html -> Text,
        freq_data -> Text,
        freq_html -> Text,
        ebt_count -> Integer,

        // === Additional fields for Simsapa ===
        dictionary_id -> Integer,
        uid -> Text,
        word_ascii -> Text,
        lemma_clean -> Text,
    }
}

diesel::table! {
    dpd_roots (root) {
        root -> Text,
        root_in_comps -> Text,
        root_has_verb -> Text,
        root_group -> Integer,
        root_sign -> Text,
        root_meaning -> Text,
        sanskrit_root -> Text,
        sanskrit_root_meaning -> Text,
        sanskrit_root_class -> Text,
        root_example -> Text,
        dhatupatha_num -> Text,
        dhatupatha_root -> Text,
        dhatupatha_pali -> Text,
        dhatupatha_english -> Text,
        dhatumanjusa_num -> Integer,
        dhatumanjusa_root -> Text,
        dhatumanjusa_pali -> Text,
        dhatumanjusa_english -> Text,
        dhatumala_root -> Text,
        dhatumala_pali -> Text,
        dhatumala_english -> Text,
        panini_root -> Text,
        panini_sanskrit -> Text,
        panini_english -> Text,
        note -> Text,
        matrix_test -> Text,
        root_info -> Text,
        root_matrix -> Text,
        // created_at -> Nullable<Timestamp>,
        // updated_at -> Nullable<Timestamp>,

        // === Additional fields for Simsapa ===
        dictionary_id -> Integer,
        uid -> Text,
        word_ascii -> Text,
        root_clean -> Text,
        root_no_sign -> Text,
    }
}

diesel::table! {
    family_compound (compound_family) {
        compound_family -> Text,
        html -> Text,
        data -> Text,
        count -> Integer,
    }
}

diesel::table! {
    family_idiom (idiom) {
        idiom -> Text,
        html -> Text,
        data -> Text,
        count -> Integer,
    }
}

diesel::table! {
    family_root (root_family_key, root_key) {
        root_family_key -> Text,
        root_key -> Text,
        root_family -> Text,
        root_meaning -> Text,
        html -> Text,
        data -> Text,
        count -> Integer,
    }
}

diesel::table! {
    family_set (set_col) { // Renamed "set" to "set_col" as "set" is a keyword
        set_col -> Text, // renamed
        html -> Text,
        data -> Text,
        count -> Integer,
    }
}

diesel::table! {
    family_word (word_family) {
        word_family -> Text,
        html -> Text,
        data -> Text,
        count -> Integer,
    }
}

diesel::table! {
    inflection_templates (pattern) {
        pattern -> Text,
        like_col -> Text, // renamed
        data -> Text,
    }
}

diesel::table! {
    lookup (lookup_key) {
        lookup_key -> Text,
        headwords -> Text,
        roots -> Text,
        deconstructor -> Text,
        variant -> Text,
        spelling -> Text,
        grammar -> Text,
        help -> Text,
        abbrev -> Text,
        epd -> Text,
        rpd -> Text,
        other -> Text,
        sinhala -> Text,
        devanagari -> Text,
        thai -> Text,
    }
}

diesel::joinable!(dpd_headwords -> dpd_roots (root_key));
diesel::joinable!(dpd_headwords -> family_word (family_word_fk));
diesel::joinable!(dpd_headwords -> inflection_templates (pattern));

diesel::allow_tables_to_appear_in_same_query!(
    bold_definitions,
    db_info,
    dpd_headwords,
    dpd_roots,
    family_compound,
    family_idiom,
    family_root,
    family_set,
    family_word,
    inflection_templates,
    lookup,
);
