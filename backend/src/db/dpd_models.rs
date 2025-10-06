use diesel::prelude::*;
// use chrono::NaiveDateTime;

use lazy_static::lazy_static;
use regex::Regex;

use crate::db::dpd_schema::*;
use crate::logger::{warn, error};

#[derive(Debug, Clone)]
pub enum UDpdWord {
    Headword(DpdHeadword),
    Root(DpdRoot),
}

impl UDpdWord {
    pub fn word(&self) -> String {
        match self {
            UDpdWord::Headword(h) => h.word(),
            UDpdWord::Root(r) => r.word(),
        }
    }
}

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, PartialEq)]
#[diesel(table_name = db_info)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct DbInfo {
    pub id: i32,
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, PartialEq)]
#[diesel(table_name = inflection_templates)]
#[diesel(primary_key(pattern))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct InflectionTemplate {
    pub pattern: String,
    pub like_col: String, // renamed
    pub data: String,
}

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, PartialEq, Eq, Hash)]
#[diesel(table_name = dpd_roots)]
#[diesel(primary_key(root))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct DpdRoot {
    pub root: String,
    pub root_in_comps: String,
    pub root_has_verb: String,
    pub root_group: i32,
    pub root_sign: String,
    pub root_meaning: String,
    pub sanskrit_root: String,
    pub sanskrit_root_meaning: String,
    pub sanskrit_root_class: String,
    pub root_example: String,
    pub dhatupatha_num: String,
    pub dhatupatha_root: String,
    pub dhatupatha_pali: String,
    pub dhatupatha_english: String,
    pub dhatumanjusa_num: i32,
    pub dhatumanjusa_root: String,
    pub dhatumanjusa_pali: String,
    pub dhatumanjusa_english: String,
    pub dhatumala_root: String,
    pub dhatumala_pali: String,
    pub dhatumala_english: String,
    pub panini_root: String,
    pub panini_sanskrit: String,
    pub panini_english: String,
    pub note: String,
    pub matrix_test: String,
    pub root_info: String,
    pub root_matrix: String,
    // pub created_at: Option<NaiveDateTime>, // removed
    // pub updated_at: Option<NaiveDateTime>, // removed

    // === Additional fields for Simsapa ===
    pub dictionary_id: i32,
    pub uid: String,
    pub word_ascii: String,
    pub root_clean: String,
    pub root_no_sign: String,
}

impl DpdRoot {
    // === Methods from Python DPD model ===

    /// Remove digits from the end
    pub fn calc_root_clean(&self) -> String {
        lazy_static! {
            static ref re_no_digits: Regex = Regex::new(r" \d.*$").unwrap();
        }
        return re_no_digits.replace_all(&self.root, "").to_string()
    }

    /// Remove digits and the root sign
    pub fn calc_root_no_sign(&self) -> String {
        lazy_static! {
            static ref re_no_root_sign: Regex = Regex::new(r"\d| |√").unwrap();
        }
        return re_no_root_sign.replace_all(&self.root, "").to_string()
    }

    // === Methods used in Simsapa ===

    pub fn word(&self) -> String {
        self.root.clone()
    }
}

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, PartialEq)]
#[diesel(table_name = family_root)]
#[diesel(primary_key(root_family_key, root_key))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct FamilyRoot {
    pub root_family_key: String,
    pub root_key: String,
    pub root_family: String,
    pub root_meaning: String,
    pub html: String,
    pub data: String,
    pub count: i32,
}

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, PartialEq)]
#[diesel(table_name = lookup)]
#[diesel(primary_key(lookup_key))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Lookup {
    pub lookup_key: String,
    pub headwords: String,
    pub roots: String,
    pub deconstructor: String,
    pub variant: String,
    pub spelling: String,
    pub grammar: String,
    pub help: String,
    pub abbrev: String,
    pub epd: String,
    pub rpd: String,
    pub other: String,
    pub sinhala: String,
    pub devanagari: String,
    pub thai: String,
}

impl Lookup {
    pub fn headwords_unpack(&self) -> Vec<i32> {
        if self.headwords.is_empty() {
            warn(&format!("headwords empty for lookup_key: {}", self.lookup_key));
            return Vec::new();
        }
        let res: Vec<i32> = match serde_json::from_str(&self.headwords) {
            Ok(x) => x,
            Err(e) => {
                error(&format!("Cannot parse headwords on lookup_key: {}\n'{}'\n'{}'",
                         e,
                         &self.lookup_key,
                         &self.headwords));
                Vec::new()
            }
        };

        res
    }

    pub fn deconstructor_unpack(&self) -> Vec<String> {
        if self.deconstructor.is_empty() {
            warn(&format!("deconstructor empty for lookup_key: {}", self.lookup_key));
            return Vec::new();
        }
        let res: Vec<String> = match serde_json::from_str(&self.deconstructor) {
            Ok(x) => x,
            Err(e) => {
                error(&format!("ERROR: Cannot parse deconstructor on lookup_key: {}\n'{}'\n'{}'",
                         e,
                         &self.lookup_key,
                         &self.deconstructor));
                Vec::new()
            }
        };

        res
    }

    // === Methods used in Simsapa ===

    /// Parses .deconstructor to a list of headword combinations (list of lists).
    ///
    /// Example:
    /// lookup_key: kammapattā
    /// deconstructor: ["kamma + pattā", "kamma + apattā", "kammi + apattā", "kammā + apattā", "kammaṁ + apattā"]
    /// return: [["kamma", "pattā"], ["kamma", "apattā"], ["kammi", "apattā"]]
    pub fn deconstructor_nested(&self) -> Vec<Vec<String>> {
        self.deconstructor_unpack()
            .into_iter()
            .map(|entry| {
                entry.split('+')
                     .map(|s| s.trim().to_string())
                     .collect()
            })
            .collect()
    }

    /// Unique deconstructor headwords as a flattened list. Preserve the order
    /// in which the deconstructor entries listed the words.
    pub fn deconstructor_flat(&self) -> Vec<String> {
        let mut res: Vec<String> = Vec::new();
        for list in self.deconstructor_nested().iter() {
            for word in list.iter() {
                if res.contains(word) {
                    continue;
                } else {
                    res.push(word.to_string());
                }
            }
        }
        res
    }
}

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, PartialEq)]
#[diesel(table_name = family_compound)]
#[diesel(primary_key(compound_family))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct FamilyCompound {
    pub compound_family: String,
    pub html: String,
    pub data: String,
    pub count: i32,
}

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, PartialEq)]
#[diesel(table_name = family_word)]
#[diesel(primary_key(word_family))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct FamilyWord {
    pub word_family: String,
    pub html: String,
    pub data: String,
    pub count: i32,
}

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, PartialEq)]
#[diesel(table_name = family_set)]
#[diesel(primary_key(set_col))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct FamilySet {
    pub set_col: String, // renamed
    pub html: String,
    pub data: String,
    pub count: i32,
}

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, PartialEq)]
#[diesel(table_name = family_idiom)]
#[diesel(primary_key(idiom))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct FamilyIdiom {
    pub idiom: String,
    pub html: String,
    pub data: String,
    pub count: i32,
}

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, PartialEq)]
#[diesel(table_name = bold_definitions)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct BoldDefinition {
    pub id: i32,
    pub file_name: String,
    pub ref_code: String,
    pub nikaya: String,
    pub book: String,
    pub title: String,
    pub subhead: String,
    pub bold: String,
    pub bold_end: String,
    pub commentary: String,
}

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, PartialEq, Associations, serde::Serialize)]
#[diesel(table_name = dpd_headwords)]
#[diesel(belongs_to(DpdRoot, foreign_key = root_key))]
#[diesel(belongs_to(FamilyWord, foreign_key = family_word_fk))]
#[diesel(belongs_to(InflectionTemplate, foreign_key = pattern))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct DpdHeadword {
    pub id: i32,
    pub lemma_1: String,
    pub lemma_2: String,
    pub pos: String,
    pub grammar: String,
    pub derived_from: String,
    pub neg: String,
    pub verb: String,
    pub trans: String,
    pub plus_case: String,
    pub meaning_1: String,
    pub meaning_lit: String,
    pub meaning_2: String,
    pub non_ia: String,
    pub sanskrit: String,
    pub root_key: String,
    pub root_sign: String,
    pub root_base: String,
    pub family_root_fk: String, // renamed
    pub family_word_fk: String, // renamed
    pub family_compound_fk: String, // renamed
    pub family_idioms_fk: String, // renamed
    pub family_set_fk: String, // renamed
    pub construction: String,
    pub derivative: String,
    pub suffix: String,
    pub phonetic: String,
    pub compound_type: String,
    pub compound_construction: String,
    pub non_root_in_comps: String,
    pub source_1: String,
    pub sutta_1: String,
    pub example_1: String,
    pub source_2: String,
    pub sutta_2: String,
    pub example_2: String,
    pub antonym: String,
    pub synonym: String,
    pub variant: String,
    pub var_phonetic: String,
    pub var_text: String,
    pub commentary: String,
    pub notes: String,
    pub cognate: String,
    pub link: String,
    pub origin: String,
    pub stem: String,
    pub pattern: String,
    // pub created_at: Option<NaiveDateTime>, // removed
    // pub updated_at: Option<NaiveDateTime>, // removed
    pub inflections: String,
    pub inflections_api_ca_eva_iti: String,
    pub inflections_sinhala: String,
    pub inflections_devanagari: String,
    pub inflections_thai: String,
    pub inflections_html: String,
    pub freq_data: String,
    pub freq_html: String,
    pub ebt_count: i32,

    // === Additional fields for Simsapa ===
    pub dictionary_id: i32,
    pub uid: String,
    pub word_ascii: String,
    pub lemma_clean: String,
}

impl DpdHeadword {
    // === Methods from Python DPD model ===

    pub fn calc_lemma_clean(&self) -> String {
        lazy_static! {
            static ref re_lemma_clean: Regex = Regex::new(r" \d.*$").unwrap();
        }
        return re_lemma_clean.replace_all(&self.lemma_1, "").to_string()
    }

    // === Methods used in Simsapa ===

    pub fn word(&self) -> String {
        self.lemma_1.clone()
    }
}

// ==========================
// === Insertable Structs ===
// ==========================

#[derive(Insertable, Debug)]
#[diesel(table_name = db_info)]
pub struct NewDbInfo<'a> {
    pub key: &'a str,
    pub value: &'a str,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = inflection_templates)]
pub struct NewInflectionTemplate<'a> {
    pub pattern: &'a str,
    pub like_col: &'a str,
    pub data: &'a str,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = dpd_roots)]
pub struct NewDpdRoot<'a> {
    pub root: &'a str,
    pub root_in_comps: &'a str,
    pub root_has_verb: &'a str,
    pub root_group: i32,
    pub root_sign: &'a str,
    pub root_meaning: &'a str,
    pub sanskrit_root: &'a str,
    pub sanskrit_root_meaning: &'a str,
    pub sanskrit_root_class: &'a str,
    pub root_example: &'a str,
    pub dhatupatha_num: &'a str,
    pub dhatupatha_root: &'a str,
    pub dhatupatha_pali: &'a str,
    pub dhatupatha_english: &'a str,
    pub dhatumanjusa_num: i32,
    pub dhatumanjusa_root: &'a str,
    pub dhatumanjusa_pali: &'a str,
    pub dhatumanjusa_english: &'a str,
    pub dhatumala_root: &'a str,
    pub dhatumala_pali: &'a str,
    pub dhatumala_english: &'a str,
    pub panini_root: &'a str,
    pub panini_sanskrit: &'a str,
    pub panini_english: &'a str,
    pub note: &'a str,
    pub matrix_test: &'a str,
    pub root_info: &'a str,
    pub root_matrix: &'a str,
    // pub created_at: Option<NaiveDateTime>,
    // pub updated_at: Option<NaiveDateTime>,

    // === Additional fields for Simsapa ===
    pub dictionary_id: i32,
    pub uid: &'a str,
    pub word_ascii: &'a str,
    pub root_clean: &'a str,
    pub root_no_sign: &'a str,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = family_root)]
pub struct NewFamilyRoot<'a> {
    pub root_family_key: &'a str,
    pub root_key: &'a str,
    pub root_family: &'a str,
    pub root_meaning: &'a str,
    pub html: &'a str,
    pub data: &'a str,
    pub count: i32,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = lookup)]
pub struct NewLookup<'a> {
    pub lookup_key: &'a str,
    pub headwords: &'a str,
    pub roots: &'a str,
    pub deconstructor: &'a str,
    pub variant: &'a str,
    pub spelling: &'a str,
    pub grammar: &'a str,
    pub help: &'a str,
    pub abbrev: &'a str,
    pub epd: &'a str,
    pub rpd: &'a str,
    pub other: &'a str,
    pub sinhala: &'a str,
    pub devanagari: &'a str,
    pub thai: &'a str,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = family_compound)]
pub struct NewFamilyCompound<'a> {
    pub compound_family: &'a str,
    pub html: &'a str,
    pub data: &'a str,
    pub count: i32,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = family_word)]
pub struct NewFamilyWord<'a> {
    pub word_family: &'a str,
    pub html: &'a str,
    pub data: &'a str,
    pub count: i32,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = family_set)]
pub struct NewFamilySet<'a> {
    pub set_col: &'a str,
    pub html: &'a str,
    pub data: &'a str,
    pub count: i32,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = family_idiom)]
pub struct NewFamilyIdiom<'a> {
    pub idiom: &'a str,
    pub html: &'a str,
    pub data: &'a str,
    pub count: i32,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = bold_definitions)]
pub struct NewBoldDefinition<'a> {
    pub file_name: &'a str,
    pub ref_code: &'a str,
    pub nikaya: &'a str,
    pub book: &'a str,
    pub title: &'a str,
    pub subhead: &'a str,
    pub bold: &'a str,
    pub bold_end: &'a str,
    pub commentary: &'a str,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = dpd_headwords)]
pub struct NewDpdHeadword<'a> {
    pub lemma_1: &'a str,
    pub lemma_2: &'a str,
    pub pos: &'a str,
    pub grammar: &'a str,
    pub derived_from: &'a str,
    pub neg: &'a str,
    pub verb: &'a str,
    pub trans: &'a str,
    pub plus_case: &'a str,
    pub meaning_1: &'a str,
    pub meaning_lit: &'a str,
    pub meaning_2: &'a str,
    pub non_ia: &'a str,
    pub sanskrit: &'a str,
    pub root_key: &'a str,
    pub root_sign: &'a str,
    pub root_base: &'a str,
    pub family_root_fk: &'a str,
    pub family_word_fk: &'a str,
    pub family_compound_fk: &'a str,
    pub family_idioms_fk: &'a str,
    pub family_set_fk: &'a str,
    pub construction: &'a str,
    pub derivative: &'a str,
    pub suffix: &'a str,
    pub phonetic: &'a str,
    pub compound_type: &'a str,
    pub compound_construction: &'a str,
    pub non_root_in_comps: &'a str,
    pub source_1: &'a str,
    pub sutta_1: &'a str,
    pub example_1: &'a str,
    pub source_2: &'a str,
    pub sutta_2: &'a str,
    pub example_2: &'a str,
    pub antonym: &'a str,
    pub synonym: &'a str,
    pub variant: &'a str,
    pub var_phonetic: &'a str,
    pub var_text: &'a str,
    pub commentary: &'a str,
    pub notes: &'a str,
    pub cognate: &'a str,
    pub link: &'a str,
    pub origin: &'a str,
    pub stem: &'a str,
    pub pattern: &'a str,
    // pub created_at: Option<NaiveDateTime>,
    // pub updated_at: Option<NaiveDateTime>,
    pub inflections: &'a str,
    pub inflections_api_ca_eva_iti: &'a str,
    pub inflections_sinhala: &'a str,
    pub inflections_devanagari: &'a str,
    pub inflections_thai: &'a str,
    pub inflections_html: &'a str,
    pub freq_data: &'a str,
    pub freq_html: &'a str,
    pub ebt_count: i32,

    // === Additional fields for Simsapa ===
    pub dictionary_id: i32,
    pub uid: &'a str,
    pub word_ascii: &'a str,
    pub lemma_clean: &'a str,
}
