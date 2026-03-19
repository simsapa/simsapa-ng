pub mod algorithms;
mod among;
mod snowball_env;

pub use among::Among;
pub use snowball_env::SnowballEnv;

use std::borrow::Cow;

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum Algorithm {
    Arabic,
    Armenian,
    Basque,
    Catalan,
    Danish,
    Dutch,
    English,
    Esperanto,
    Estonian,
    Finnish,
    French,
    German,
    Greek,
    Hindi,
    Hungarian,
    Indonesian,
    Irish,
    Italian,
    Lithuanian,
    Nepali,
    Norwegian,
    Pali,
    Polish,
    Portuguese,
    Romanian,
    Russian,
    Serbian,
    Spanish,
    Swedish,
    Tamil,
    Turkish,
    Yiddish,
}

#[derive(Clone)]
pub struct Stemmer {
    stemmer: fn(&mut SnowballEnv) -> bool,
}

impl Stemmer {
    pub fn create(algo: Algorithm) -> Self {
        match algo {
            Algorithm::Arabic => Stemmer { stemmer: algorithms::arabic_stemmer::stem },
            Algorithm::Armenian => Stemmer { stemmer: algorithms::armenian_stemmer::stem },
            Algorithm::Basque => Stemmer { stemmer: algorithms::basque_stemmer::stem },
            Algorithm::Catalan => Stemmer { stemmer: algorithms::catalan_stemmer::stem },
            Algorithm::Danish => Stemmer { stemmer: algorithms::danish_stemmer::stem },
            Algorithm::Dutch => Stemmer { stemmer: algorithms::dutch_stemmer::stem },
            Algorithm::English => Stemmer { stemmer: algorithms::english_stemmer::stem },
            Algorithm::Esperanto => Stemmer { stemmer: algorithms::esperanto_stemmer::stem },
            Algorithm::Estonian => Stemmer { stemmer: algorithms::estonian_stemmer::stem },
            Algorithm::Finnish => Stemmer { stemmer: algorithms::finnish_stemmer::stem },
            Algorithm::French => Stemmer { stemmer: algorithms::french_stemmer::stem },
            Algorithm::German => Stemmer { stemmer: algorithms::german_stemmer::stem },
            Algorithm::Greek => Stemmer { stemmer: algorithms::greek_stemmer::stem },
            Algorithm::Hindi => Stemmer { stemmer: algorithms::hindi_stemmer::stem },
            Algorithm::Hungarian => Stemmer { stemmer: algorithms::hungarian_stemmer::stem },
            Algorithm::Indonesian => Stemmer { stemmer: algorithms::indonesian_stemmer::stem },
            Algorithm::Irish => Stemmer { stemmer: algorithms::irish_stemmer::stem },
            Algorithm::Italian => Stemmer { stemmer: algorithms::italian_stemmer::stem },
            Algorithm::Lithuanian => Stemmer { stemmer: algorithms::lithuanian_stemmer::stem },
            Algorithm::Nepali => Stemmer { stemmer: algorithms::nepali_stemmer::stem },
            Algorithm::Norwegian => Stemmer { stemmer: algorithms::norwegian_stemmer::stem },
            Algorithm::Pali => Stemmer { stemmer: algorithms::pali_stemmer::stem },
            Algorithm::Polish => Stemmer { stemmer: algorithms::polish_stemmer::stem },
            Algorithm::Portuguese => Stemmer { stemmer: algorithms::portuguese_stemmer::stem },
            Algorithm::Romanian => Stemmer { stemmer: algorithms::romanian_stemmer::stem },
            Algorithm::Russian => Stemmer { stemmer: algorithms::russian_stemmer::stem },
            Algorithm::Serbian => Stemmer { stemmer: algorithms::serbian_stemmer::stem },
            Algorithm::Spanish => Stemmer { stemmer: algorithms::spanish_stemmer::stem },
            Algorithm::Swedish => Stemmer { stemmer: algorithms::swedish_stemmer::stem },
            Algorithm::Tamil => Stemmer { stemmer: algorithms::tamil_stemmer::stem },
            Algorithm::Turkish => Stemmer { stemmer: algorithms::turkish_stemmer::stem },
            Algorithm::Yiddish => Stemmer { stemmer: algorithms::yiddish_stemmer::stem },
        }
    }

    pub fn stem<'a>(&self, input: &'a str) -> Cow<'a, str> {
        let mut env = SnowballEnv::create(input);
        (self.stemmer)(&mut env);
        env.get_current()
    }
}

/// Map a language code to the appropriate stemming algorithm.
/// Returns English as the fallback for unknown language codes.
pub fn lang_to_algorithm(lang_code: &str) -> Algorithm {
    match lang_code {
        "pli" => Algorithm::Pali,
        "san" => Algorithm::Pali,
        "ar" => Algorithm::Arabic,
        "hy" => Algorithm::Armenian,
        "eu" => Algorithm::Basque,
        "ca" => Algorithm::Catalan,
        "da" => Algorithm::Danish,
        "nl" => Algorithm::Dutch,
        "en" => Algorithm::English,
        "eo" => Algorithm::Esperanto,
        "et" => Algorithm::Estonian,
        "fi" => Algorithm::Finnish,
        "fr" => Algorithm::French,
        "de" => Algorithm::German,
        "el" => Algorithm::Greek,
        "hi" => Algorithm::Hindi,
        "hu" => Algorithm::Hungarian,
        "id" => Algorithm::Indonesian,
        "ga" => Algorithm::Irish,
        "it" => Algorithm::Italian,
        "lt" => Algorithm::Lithuanian,
        "ne" => Algorithm::Nepali,
        "no" => Algorithm::Norwegian,
        "pl" => Algorithm::Polish,
        "pt" => Algorithm::Portuguese,
        "ro" => Algorithm::Romanian,
        "ru" => Algorithm::Russian,
        "sr" => Algorithm::Serbian,
        "es" => Algorithm::Spanish,
        "sv" => Algorithm::Swedish,
        "ta" => Algorithm::Tamil,
        "tr" => Algorithm::Turkish,
        "yi" => Algorithm::Yiddish,
        _ => Algorithm::English,
    }
}
