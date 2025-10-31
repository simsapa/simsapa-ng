//! Database insertion logic for parsed suttas

use anyhow::{Context, Result};
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use simsapa_backend::db::appdata_models::NewSutta;
use simsapa_backend::db::appdata_schema::suttas;
use simsapa_backend::logger;
use crate::tipitaka_xml_parser_tsv::types::*;

/// Insert a sutta into the database
pub fn insert_sutta(
    conn: &mut SqliteConnection,
    sutta: &Sutta,
    content_html: &str,
    content_plain: &str,
) -> Result<()> {
    let new_sutta = NewSutta {
        uid: &sutta.metadata.uid,
        sutta_ref: &sutta.metadata.sutta_ref,
        nikaya: &sutta.metadata.nikaya,
        language: "pli",
        group_path: Some(&sutta.metadata.group_path),
        group_index: sutta.metadata.group_index,
        order_index: sutta.metadata.order_index,
        sutta_range_group: None,
        sutta_range_start: None,
        sutta_range_end: None,
        title: Some(&sutta.title),
        title_ascii: None,
        title_pali: Some(&sutta.title),
        title_trans: None,
        description: None,
        content_plain: Some(content_plain),
        content_html: Some(content_html),
        content_json: None,
        content_json_tmpl: None,
        source_uid: Some("cst4"),
        source_info: Some("VRI CST Tipitaka"),
        source_language: Some("pli"),
        message: None,
        copyright: Some("VRI"),
        license: None,
    };

    diesel::insert_into(suttas::table)
        .values(&new_sutta)
        .execute(conn)
        .map_err(|e| {
            anyhow::anyhow!("Failed to insert sutta {}: Diesel error: {}", sutta.metadata.uid, e)
        })?;

    Ok(())
}

/// Insert multiple suttas in a transaction
pub fn insert_suttas_batch(
    conn: &mut SqliteConnection,
    suttas: &[(Sutta, String, String)], // (sutta, html, plain)
) -> Result<usize> {
    conn.transaction(|conn| {
        let mut inserted = 0;
        
        for (sutta, html, plain) in suttas {
            match insert_sutta(conn, sutta, html, plain) {
                Ok(_) => inserted += 1,
                Err(e) => {
                    logger::warn(&format!("Failed to insert sutta {}: {:?}", sutta.metadata.uid, e));
                }
            }
        }
        
        Ok(inserted)
    })
}
