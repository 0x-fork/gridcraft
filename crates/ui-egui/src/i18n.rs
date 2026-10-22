//! Interface translations. Command ids and document text stay stable; only interface chrome is
//! translated. Untranslated labels fall back to English, so coverage can grow incrementally.
//!
//! The interface language is a preference ([`crate::UiState::language`]): on first run it defaults
//! to the system's language ([`Language::system`]), and the View tab switches it by hand.
//!
//! CJK fonts stay system-provided: [`crate::theme`] appends available CJK faces to every family so
//! Japanese, Chinese and Korean labels and workbook text can render without bundling font files.

use serde::{Deserialize, Serialize};

mod cs;
mod de;
mod el;
mod es;
mod fr;
mod it;
mod ja;
mod ko;
mod pl;
mod nl;
mod pt;
mod ru;
mod sr;
mod vi;
mod zh;
mod uk;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    En,
    Es,
    El,
    Zh,
    Ja,
    Ko,
    Ru,
    #[serde(rename = "pt")]
    PtBr,
    De,
    Cs,
    Fr,
    Sr,
    Pl,
    It,
    Uk,
    Vi,
    Nl,
}

impl Language {
    pub const ALL: [Self; 17] = [
        Self::En, Self::Es, Self::El, Self::Zh, Self::Ja, Self::Ko, Self::Ru, Self::PtBr, Self::De, Self::Cs, Self::Fr, Self::Sr, Self::Pl, Self::It, Self::Uk, Self::Vi, Self::Nl,
    ];

    /// The language's own name, shown in the switcher.
    pub fn name(self) -> &'static str {
        match self {
            Self::En => "English",
            Self::Es => "Español",
            Self::El => "Ελληνικά",
            Self::Zh => "简体中文",
            Self::Ja => "日本語",
            Self::Ko => "한국어",
            Self::Ru => "Русский",
            Self::PtBr => "Português (Brasil)",
            Self::De => "Deutsch",
            Self::Cs => "Čeština",
            Self::Fr => "Français",
            Self::Sr => "Srpski",
            Self::Pl => "Polski",
            Self::It => "Italiano",
            Self::Uk => "Українська",
            Self::Vi => "Tiếng Việt",
            Self::Nl => "Nederlands",
        }
    }

    /// The language's code, as saved in `ui.json` and taken by `app.language.set`.
    pub const fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Es => "es",
            Self::El => "el",
            Self::Zh => "zh",
            Self::Ja => "ja",
            Self::Ko => "ko",
            Self::Ru => "ru",
            Self::PtBr => "pt",
            Self::De => "de",
            Self::Cs => "cs",
            Self::Fr => "fr",
            Self::Sr => "sr",
            Self::Pl => "pl",
            Self::It => "it",
            Self::Uk => "uk",
            Self::Vi => "vi",
            Self::Nl => "nl",
        }
    }

    pub fn parse(code: &str) -> Option<Self> {
        Self::from_tag(code)
    }

    fn from_tag(tag: &str) -> Option<Self> {
        match tag.trim().split(['-', '_']).next()?.to_ascii_lowercase().as_str() {
            "en" => Some(Self::En),
            "es" => Some(Self::Es),
            "el" => Some(Self::El),
            "zh" => Some(Self::Zh),
            "ja" => Some(Self::Ja),
            "ko" => Some(Self::Ko),
            "ru" => Some(Self::Ru),
            "pt" => Some(Self::PtBr),
            "de" => Some(Self::De),
            "cs" => Some(Self::Cs),
            "fr" => Some(Self::Fr),
            "sr" => Some(Self::Sr),
            "pl" => Some(Self::Pl),
            "it" => Some(Self::It),
            "uk" => Some(Self::Uk),
            "vi" => Some(Self::Vi),
            "nl" => Some(Self::Nl),
            _ => None,
        }
    }

    /// The system's preferred interface language, best effort; English when it is unknown or
    /// unsupported. Used as the first-run default while nothing is saved.
    pub fn system() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        {
            sys_locale::get_locale().and_then(|tag| Self::from_tag(&tag)).unwrap_or(Self::En)
        }
        #[cfg(target_arch = "wasm32")]
        {
            web_sys::window().and_then(|window| window.navigator().language()).and_then(|tag| Self::from_tag(&tag)).unwrap_or(Self::En)
        }
    }

    pub fn formula_locale(self) -> gridcraft_engine::formula::FormulaLocale {
        if self == Self::Es { gridcraft_engine::formula::FormulaLocale::Es } else { gridcraft_engine::formula::FormulaLocale::En }
    }

    /// Translates an interface label; untranslated labels fall back to English.
    ///
    /// A label that needs two translations (Home's text "Orientation" vs Page Layout's print
    /// "Orientation") is keyed `context|label`: English shows only the part after the `|`.
    pub fn tr(self, text: &str) -> &str {
        let translated = match self {
            Self::En => None,
            Self::Es => es::translate(text),
            Self::El => el::translate(text),
            Self::Ja => ja::translate(text),
            Self::Zh => zh::translate(text),
            Self::Ko => ko::translate(text),
            Self::Ru => ru::translate(text),
            Self::PtBr => pt::translate(text),
            Self::De => de::translate(text),
            Self::Cs => cs::translate(text),
            Self::Fr => fr::translate(text),
            Self::Sr => sr::translate(text),
            Self::Pl => pl::translate(text),
            Self::It => it::translate(text),
            Self::Uk => uk::translate(text),
            Self::Vi => vi::translate(text),
            Self::Nl => nl::translate(text),
        };
        translated.unwrap_or_else(|| text.rsplit_once('|').map_or(text, |(_, label)| label))
    }
}

/// Deserializes a saved language leniently: an unknown or malformed value (a language removed in a
/// later version, a hand-edited `ui.json`) reads as English instead of failing the whole `UiState`,
/// which would reset every other preference with it.
pub fn lenient<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Language, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Raw {
        Code(String),
        Other(serde::de::IgnoredAny),
    }
    Ok(match Raw::deserialize(d)? {
        Raw::Code(code) => Language::parse(&code).unwrap_or_default(),
        Raw::Other(_) => Language::default(),
    })
}

/// The language a saved `ui.json` asks for, if it names a supported one. The desktop app uses it
/// to tell "saved English" from "nothing usable saved" (first run, an older file, an unknown
/// value), which starts from [`Language::system`] instead.
pub fn saved_language(ui_json: &serde_json::Value) -> Option<Language> {
    ui_json.get("language").and_then(serde_json::Value::as_str).and_then(Language::parse)
}

/// egui context-data key holding the language of the frame being drawn. Widgets that only get a
/// `Ui` (not the app) read it to translate their labels; [`set_current`] writes it once per frame.
fn lang_key() -> egui::Id {
    egui::Id::new("gridcraft.language")
}

/// The language the current frame is drawing in. `En` when unset.
pub fn current(ctx: &egui::Context) -> Language {
    ctx.data(|d| d.get_temp(lang_key())).unwrap_or_default()
}

/// Sets the language for the current frame; call once per frame before anything is drawn.
pub fn set_current(ctx: &egui::Context, language: Language) {
    ctx.data_mut(|d| d.insert_temp(lang_key(), language));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translations_are_unique_and_preserve_unknown_text() {
        for (i, (en, ja)) in ja::CORE.iter().enumerate() {
            assert!(!ja.is_empty());
            assert!(ja::CORE.iter().take(i).all(|(other, _)| en != other));
            assert_eq!(Language::En.tr(en), en.rsplit_once('|').map_or(*en, |(_, label)| label));
        }
        for (english, _) in ja::CORE {
            assert!(zh::translate(english).is_some_and(|text| !text.is_empty()), "missing Simplified Chinese: {english}");
            assert!(ko::translate(english).is_some_and(|text| !text.is_empty()), "missing Korean: {english}");
            assert!(ru::translate(english).is_some_and(|text| !text.is_empty()), "missing Russian: {english}");
            assert!(pt::translate(english).is_some_and(|text| !text.is_empty()), "missing Portuguese: {english}");
            assert!(es::translate(english).is_some_and(|text| !text.is_empty()), "missing Spanish: {english}");
            assert!(el::translate(english).is_some_and(|text| !text.is_empty()), "missing Greek: {english}");
            assert!(de::translate(english).is_some_and(|text| !text.is_empty()), "missing German: {english}");
            assert!(cs::translate(english).is_some_and(|text| !text.is_empty()), "missing Czech: {english}");
            assert!(fr::translate(english).is_some_and(|text| !text.is_empty()), "missing French: {english}");
            assert!(sr::translate(english).is_some_and(|text| !text.is_empty()), "missing Serbian: {english}");
            assert!(pl::translate(english).is_some_and(|text| !text.is_empty()), "missing Polish: {english}");
            assert!(it::translate(english).is_some_and(|text| !text.is_empty()), "missing Italian: {english}");
            assert!(uk::translate(english).is_some_and(|text| !text.is_empty()), "missing Ukrainian: {english}");
            assert!(vi::translate(english).is_some_and(|text| !text.is_empty()), "missing Vietnamese: {english}");
            assert!(nl::translate(english).is_some_and(|text| !text.is_empty()), "missing Dutch: {english}");
        }
        assert_eq!(Language::Ja.tr("Data"), "データ");
        assert_eq!(Language::Zh.tr("Data"), "数据");
        assert_eq!(Language::Ko.tr("Data"), "데이터");
        assert_eq!(Language::Ru.tr("Data"), "Данные");
        assert_eq!(Language::PtBr.tr("Data"), "Dados");
        assert_eq!(Language::Es.tr("Data"), "Datos");
        assert_eq!(Language::El.tr("Data"), "Δεδομένα");
        assert_eq!(Language::El.tr("Page Layout|Orientation"), "Προσανατολισμός σελίδας");
        assert_eq!(Language::El.tr("Sheet1!A1"), "Sheet1!A1");
        assert_eq!(Language::El.tr("Page {page} of {pages}"), "Σελίδα {page} από {pages}");
        assert_eq!(Language::Es.tr("Freeze Panes"), "Inmovilizar paneles");
        assert_eq!(Language::Es.tr("Sheet1!A1"), "Sheet1!A1");
        assert_eq!(Language::De.tr("Data"), "Daten");
        assert_eq!(Language::De.tr("Sheet1!A1"), "Sheet1!A1");
        assert_eq!(Language::Fr.tr("Data"), "Données");
        assert_eq!(Language::Fr.tr("Sheet1!A1"), "Sheet1!A1");
        assert_eq!(Language::PtBr.tr("Date"), "Data");
        assert_eq!(Language::PtBr.tr("Sheet1!A1"), "Sheet1!A1");
        assert_eq!(Language::Ja.tr("Sheet1!A1"), "Sheet1!A1");
        assert_eq!(Language::Zh.tr("Sheet1!A1"), "Sheet1!A1");
        assert_eq!(Language::Ko.tr("Sheet1!A1"), "Sheet1!A1");
        assert_eq!(Language::Ru.tr("Sheet1!A1"), "Sheet1!A1");
        // Czech "Data" is spelled as in English, so probe it with a label that differs.
        assert_eq!(Language::Cs.tr("Data"), "Data");
        assert_eq!(Language::Cs.tr("Formulas"), "Vzorce");
        assert_eq!(Language::Cs.tr("Sheet1!A1"), "Sheet1!A1");
        // A `context|label` key shows only its label in English and splits the translation.
        assert_eq!(Language::En.tr("Page Layout|Orientation"), "Orientation");
        assert_eq!(Language::Ja.tr("Page Layout|Orientation"), "印刷の向き");
        assert_eq!(Language::Ja.tr("Orientation"), "方向");
        assert_eq!(Language::Cs.tr("Page Layout|Orientation"), "Orientace");
        assert_eq!(Language::Cs.tr("Orientation"), "Orientace textu");
        assert_eq!(Language::Sr.tr("Data"), "Podaci");
        assert_eq!(Language::Sr.tr("Sheet1!A1"), "Sheet1!A1");
        assert_eq!(Language::Sr.tr("Page Layout|Orientation"), "Orijentacija");
        assert_eq!(Language::Sr.tr("Orientation"), "Orijentacija");
        assert_eq!(Language::Pl.tr("Data"), "Dane");
        assert_eq!(Language::Pl.tr("Sheet1!A1"), "Sheet1!A1");
        assert_eq!(Language::Pl.tr("Page Layout|Orientation"), "Orientacja");
        assert_eq!(Language::It.tr("Data"), "Dati");
        assert_eq!(Language::It.tr("Sheet1!A1"), "Sheet1!A1");
        assert_eq!(Language::It.tr("Freeze Panes"), "Blocca riquadri");
        assert_eq!(Language::It.tr("Page Layout|Orientation"), "Orientamento");
        assert_eq!(Language::It.tr("Orientation"), "Orientamento");
        assert_eq!(Language::Uk.tr("Data"), "Дані");
        assert_eq!(Language::Uk.tr("Sheet1!A1"), "Sheet1!A1");
        assert_eq!(Language::Vi.tr("Data"), "Dữ liệu");
        assert_eq!(Language::Vi.tr("Sheet1!A1"), "Sheet1!A1");
        assert_eq!(Language::Vi.tr("Page Layout|Orientation"), "Hướng giấy");
        assert_eq!(Language::Vi.tr("Orientation"), "Hướng chữ");
        assert_eq!(Language::Nl.tr("Data"), "Gegevens");
        assert_eq!(Language::Nl.tr("Sheet1!A1"), "Sheet1!A1");
        assert_eq!(Language::Nl.tr("Page Layout|Orientation"), "Paginastand");
        assert_eq!(Language::Nl.tr("Orientation"), "Richting");
        assert_eq!(Language::parse("xx"), None);
    }

    /// A catalog whose translations slip onto their neighbours' keys still covers every key, so
    /// check the shape of each translation against its English: placeholders survive, and a
    /// one-line label (a menu item, a tooltip) doesn't turn into a two-line ribbon label. French
    /// once shipped with over a hundred translations shifted this way ("Close" read "Société").
    #[test]
    fn translations_keep_the_shape_of_their_english() {
        let placeholders =
            |s: &str| s.split('{').skip(1).filter_map(|rest| rest.split_once('}').map(|(name, _)| name.to_string())).collect::<Vec<_>>();
        for language in Language::ALL {
            for (english, _) in ja::CORE {
                let label = english.rsplit_once('|').map_or(*english, |(_, label)| label);
                let text = language.tr(english);
                assert_eq!(placeholders(text), placeholders(label), "{language:?}: {english:?} => {text:?}");
                // Big ribbon buttons whose English fits on one line may still need two.
                if !label.contains('\n') && !["AutoSum", "PivotTable"].contains(english) {
                    assert!(!text.contains('\n'), "{language:?}: one-line {english:?} => {text:?}");
                }
            }
        }
        for (english, french) in [
            ("Close", "Fermer"),
            ("File", "Fichier"),
            ("Save", "Enregistrer"),
            ("Animations", "Animations"),
            ("Company", "Société"),
            ("Show Detail", "Afficher le détail"),
            ("Solver", "Solveur"),
        ] {
            assert_eq!(Language::Fr.tr(english), french);
        }
    }

    #[test]
    fn german_cell_and_header_context_menus_are_fully_translated() {
        for label in [
            "Cut",
            "Copy",
            "Paste",
            "Paste Special…",
            "Insert…",
            "Delete…",
            "Clear Contents",
            "Filter by Selected Cell's Value",
            "Sort A to Z",
            "Sort Z to A",
            "New Comment",
            "New Note",
            "Format Cells…",
            "Pick From Drop-down List…",
            "Define Name…",
            "Insert Copied Cells",
            "Insert Copied Cells…",
            "Insert Cut Cells",
            "Insert",
            "Delete",
            "Row Height…",
            "AutoFit Row Height",
            "Column Width…",
            "AutoFit Column Width",
            "Hide",
            "Unhide",
            "Group",
        ] {
            assert_ne!(Language::De.tr(label), label, "German context menu label missing: {label}");
        }
    }

    #[test]
    fn translations_never_show_the_context_prefix() {
        // `tr` returns a found translation as it is, so a catalog that keeps `context|` in its
        // value shows it on screen (File › Options read "Options|Général" in French).
        for language in Language::ALL {
            for (english, _) in ja::CORE {
                let shown = language.tr(english);
                assert!(!shown.contains('|'), "{} shows a context prefix for {english}: {shown}", language.code());
            }
        }
        assert_eq!(Language::Fr.tr("Options|General"), "Général");
        assert_eq!(Language::Fr.tr("Options|Save"), "Enregistrer");
        assert_eq!(Language::Fr.tr("Sheet Options|View"), "Affichage");
        assert_eq!(Language::Sr.tr("Options|General"), "Opšte");
        assert_eq!(Language::Sr.tr("Options|Save"), "Sačuvaj");
        assert_eq!(Language::Sr.tr("Sheet Options|View"), "Prikaz");
        assert_eq!(Language::Nl.tr("Options|General"), "Algemeen");
        assert_eq!(Language::Nl.tr("Options|Save"), "Opslaan");
        assert_eq!(Language::Nl.tr("Sheet Options|View"), "Weergeven");
    }

    #[test]
    fn unknown_saved_language_keeps_the_other_preferences() {
        let ui: crate::UiState = serde_json::from_str(r#"{"dark": true, "language": "xx"}"#).unwrap();
        assert!(ui.dark);
        assert_eq!(ui.language, Language::En);
        let ui: crate::UiState = serde_json::from_str(r#"{"dark": true, "language": 7}"#).unwrap();
        assert!(ui.dark);
        let ui: crate::UiState = serde_json::from_str(r#"{"language": "ja"}"#).unwrap();
        assert_eq!(ui.language, Language::Ja);
        assert_eq!(crate::UiState::default().language, Language::En, "the default never reads the host locale");
        assert_eq!(saved_language(&serde_json::json!({"language": "ja"})), Some(Language::Ja));
        assert_eq!(saved_language(&serde_json::json!({"language": "xx"})), None);
        assert_eq!(saved_language(&serde_json::json!({})), None);
    }

    #[test]
    fn tags_reduce_to_a_supported_language() {
        assert_eq!(Language::from_tag("ja-JP"), Some(Language::Ja));
        assert_eq!(Language::from_tag("en_US"), Some(Language::En));
        assert_eq!(Language::from_tag("es_MX"), Some(Language::Es));
        assert_eq!(Language::from_tag("el-GR"), Some(Language::El));
        assert_eq!(Language::from_tag("el_GR.UTF-8"), Some(Language::El));
        assert_eq!(Language::parse("el"), Some(Language::El));
        assert_eq!(Language::from_tag("zh-Hans-CN"), Some(Language::Zh));
        assert_eq!(Language::from_tag("ko-KR"), Some(Language::Ko));
        assert_eq!(Language::from_tag("RU_ru"), Some(Language::Ru));
        assert_eq!(Language::from_tag("de-DE"), Some(Language::De));
        assert_eq!(Language::from_tag("de_AT"), Some(Language::De));
        assert_eq!(Language::from_tag("de-CH"), Some(Language::De));
        assert_eq!(Language::from_tag("pt-BR"), Some(Language::PtBr));
        assert_eq!(Language::parse("pt"), Some(Language::PtBr));
        assert_eq!(Language::parse("pt-br"), Some(Language::PtBr));
        assert_eq!(Language::from_tag("cs-CZ"), Some(Language::Cs));
        assert_eq!(Language::from_tag("cs_CZ.UTF-8"), Some(Language::Cs));
        assert_eq!(Language::parse("cs"), Some(Language::Cs));
        assert_eq!(Language::from_tag("fr-FR"), Some(Language::Fr));
        assert_eq!(Language::from_tag("fr_CA"), Some(Language::Fr));
        assert_eq!(Language::parse("fr"), Some(Language::Fr));
        assert_eq!(Language::from_tag("sr-Latn-RS"), Some(Language::Sr));
        assert_eq!(Language::from_tag("sr-RS"), Some(Language::Sr));
        assert_eq!(Language::parse("sr"), Some(Language::Sr));
        assert_eq!(Language::from_tag("pl-PL"), Some(Language::Pl));
        assert_eq!(Language::from_tag("pl_PL.UTF-8"), Some(Language::Pl));
        assert_eq!(Language::parse("pl"), Some(Language::Pl));
        assert_eq!(Language::from_tag("it-IT"), Some(Language::It));
        assert_eq!(Language::from_tag("it_CH.UTF-8"), Some(Language::It));
        assert_eq!(Language::parse("it"), Some(Language::It));
        assert_eq!(Language::from_tag("uk-XX"), Some(Language::Uk));
        assert_eq!(Language::parse("uk"), Some(Language::Uk));
        assert_eq!(Language::from_tag("vi-VN"), Some(Language::Vi));
        assert_eq!(Language::parse("vi"), Some(Language::Vi));
        assert_eq!(Language::from_tag("nl-NL"), Some(Language::Nl));
        assert_eq!(Language::from_tag("nl_BE"), Some(Language::Nl));
        assert_eq!(Language::parse("nl"), Some(Language::Nl));
    }

    #[test]
    fn an_unset_language_preference_is_english_until_the_app_applies_the_system_one() {
        // Deserializing never reads the host locale (tests stay deterministic); the desktop app
        // applies `Language::system()` itself when `ui.json` names no usable language.
        let restored: crate::UiState = serde_json::from_str("{}").unwrap();
        assert_eq!(restored.language, Language::En);
        assert_eq!(saved_language(&serde_json::json!({"language": "zh"})), Some(Language::Zh));
    }

    #[test]
    fn language_commands_switch_and_persist() {
        let mut app = crate::SheetApp::new(gridcraft_engine::Session::default(), Default::default());
        for language in Language::ALL {
            app.run("app.language.set", serde_json::json!({"language": language.code()})).unwrap();
            assert_eq!(app.ui.language, language);
            let restored: crate::UiState = serde_json::from_str(&serde_json::to_string(&app.ui).unwrap()).unwrap();
            assert_eq!(restored.language, language);
        }
        app.run("app.language.set", serde_json::json!({"code": "ja"})).unwrap();
        assert_eq!(app.ui.language, Language::Ja, "`code` is an alias for `language`");
        assert!(app.run("app.language.set", serde_json::json!({})).is_err(), "no language is an error, not a dialog");
        app.run("app.language.english", serde_json::json!({})).unwrap();
        assert_eq!(app.ui.language, Language::En);
        for l in Language::ALL {
            assert_eq!(Language::parse(l.code()), Some(l));
        }
    }
}
