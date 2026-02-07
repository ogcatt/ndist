use dioxus_i18n::{prelude::*, *};
use unic_langid::{LanguageIdentifier, langid};

pub fn config(initial_language: LanguageIdentifier) -> I18nConfig {
    I18nConfig::new(initial_language)
        .with_locale((langid!("en-US"), include_str!("../data/i18n/en-US.ftl")))
        .with_fallback(langid!("en-US"))
}
