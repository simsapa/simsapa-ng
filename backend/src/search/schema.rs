use tantivy::schema::{IndexRecordOption, Schema, TextFieldIndexing, TextOptions};

/// Build the Tantivy schema for sutta indexing with the given language.
pub fn build_sutta_schema(lang: &str) -> Schema {
    let mut builder = Schema::builder();

    let raw_opts = TextOptions::default()
        .set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer("raw")
                .set_index_option(IndexRecordOption::Basic),
        )
        .set_stored();

    let simple_fold_opts = TextOptions::default()
        .set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer("simple_fold")
                .set_index_option(IndexRecordOption::WithFreqsAndPositions),
        )
        .set_stored();

    let lang_stem_opts = TextOptions::default()
        .set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer(&format!("{lang}_stem"))
                .set_index_option(IndexRecordOption::WithFreqsAndPositions),
        )
        .set_stored();

    let lang_normalize_opts = TextOptions::default()
        .set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer(&format!("{lang}_normalize"))
                .set_index_option(IndexRecordOption::WithFreqsAndPositions),
        )
        .set_stored();

    builder.add_text_field("uid", simple_fold_opts.clone());
    builder.add_text_field("title", simple_fold_opts.clone());
    builder.add_text_field("language", raw_opts.clone());
    builder.add_text_field("source_uid", raw_opts.clone());
    builder.add_text_field("sutta_ref", simple_fold_opts);
    builder.add_text_field("nikaya", raw_opts);
    builder.add_text_field("content", lang_stem_opts);
    builder.add_text_field("content_exact", lang_normalize_opts);

    builder.build()
}

/// Build the Tantivy schema for dictionary word indexing with the given language.
pub fn build_dict_schema(lang: &str) -> Schema {
    let mut builder = Schema::builder();

    let raw_opts = TextOptions::default()
        .set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer("raw")
                .set_index_option(IndexRecordOption::Basic),
        )
        .set_stored();

    let simple_fold_opts = TextOptions::default()
        .set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer("simple_fold")
                .set_index_option(IndexRecordOption::WithFreqsAndPositions),
        )
        .set_stored();

    let lang_stem_opts = TextOptions::default()
        .set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer(&format!("{lang}_stem"))
                .set_index_option(IndexRecordOption::WithFreqsAndPositions),
        )
        .set_stored();

    let lang_normalize_opts = TextOptions::default()
        .set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer(&format!("{lang}_normalize"))
                .set_index_option(IndexRecordOption::WithFreqsAndPositions),
        )
        .set_stored();

    builder.add_text_field("uid", simple_fold_opts.clone());
    builder.add_text_field("word", simple_fold_opts.clone());
    builder.add_text_field("synonyms", simple_fold_opts);
    builder.add_text_field("language", raw_opts.clone());
    builder.add_text_field("source_uid", raw_opts);
    builder.add_text_field("content", lang_stem_opts);
    builder.add_text_field("content_exact", lang_normalize_opts);

    builder.build()
}
