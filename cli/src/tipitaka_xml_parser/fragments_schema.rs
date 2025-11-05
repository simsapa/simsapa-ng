// @generated automatically by Diesel CLI.

diesel::table! {
    nikaya_structures (id) {
        id -> Integer,
        nikaya -> Text,
        levels -> Text,
        created_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    xml_fragments (id) {
        id -> Integer,
        cst_file -> Text,
        frag_idx -> Integer,
        frag_type -> Text,
        frag_review -> Nullable<Text>,
        nikaya -> Text,
        cst_code -> Nullable<Text>,
        sc_code -> Nullable<Text>,
        content -> Text,
        cst_vagga -> Nullable<Text>,
        cst_sutta -> Nullable<Text>,
        cst_paranum -> Nullable<Text>,
        sc_sutta -> Nullable<Text>,
        start_line -> Integer,
        start_char -> Integer,
        end_line -> Integer,
        end_char -> Integer,
        group_levels -> Text,
        created_at -> Nullable<Timestamp>,
    }
}

// Note: We don't use joinable! here because the foreign key references
// nikaya_structures.nikaya (a unique field) rather than the primary key (id).
// The FOREIGN KEY constraint is enforced at the database level.

diesel::allow_tables_to_appear_in_same_query!(
    nikaya_structures,
    xml_fragments,
);
