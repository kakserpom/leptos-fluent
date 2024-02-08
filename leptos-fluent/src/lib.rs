mod localstorage;
mod url;

use core::str::FromStr;
use fluent_templates::{
    fluent_bundle::FluentValue, loader::Loader, LanguageIdentifier,
    StaticLoader,
};
use leptos::{
    expect_context, provide_context, window, RwSignal, SignalGet,
    SignalGetUntracked, SignalSet,
};
pub use leptos_fluent_macros::leptos_fluent;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::rc::Rc;

/// Languages supported by the application
#[derive(Clone, Debug)]
pub struct Language {
    /// Language identifier
    ///
    /// Can be any valid language tag, such as `en`, `es`, `en-US`, `es-ES`, etc.
    pub id: LanguageIdentifier,
    /// Language name
    ///
    /// The name of the language, such as `English`, `Español`, etc.
    /// This name will be intended to be displayed in the language selector,
    /// but can also be used to translate it itself to other languages.
    pub name: &'static str,
}

impl PartialEq for Language {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

/// Signal for the current language
#[derive(Copy, Clone)]
pub struct LanguageSignal(pub RwSignal<&'static Language>);

impl SignalGet for LanguageSignal {
    type Value = &'static Language;

    fn get(&self) -> Self::Value {
        self.0.get()
    }

    fn try_get(&self) -> Option<Self::Value> {
        self.0.try_get()
    }
}

impl SignalGetUntracked for LanguageSignal {
    type Value = &'static Language;

    fn get_untracked(&self) -> Self::Value {
        self.0.get_untracked()
    }

    fn try_get_untracked(&self) -> Option<Self::Value> {
        self.0.try_get_untracked()
    }
}

impl SignalSet for LanguageSignal {
    type Value = &'static Language;

    fn set(&self, value: Self::Value) {
        self.0.set(value);
    }

    fn try_set(&self, value: Self::Value) -> Option<Self::Value> {
        self.0.try_set(value)
    }
}

/// Internationalization context
///
/// This context is used to provide the current language, the available languages
/// and all the translations. It is capable of doing what is needed to translate
/// and manage translations in a whole application.
///
/// If you need to separate the translations of different parts of the application,
/// you can wrap this context in another struct and provide it to Leptos as a context.
pub struct I18n {
    /// Signal that holds the current language
    pub language: Rc<LanguageSignal>,
    /// Available languages for the application
    pub languages: &'static [&'static Language],
    pub locales: &'static Lazy<StaticLoader>,
    pub initial_language_from_url: bool,
    pub initial_language_from_url_param: &'static str,
    pub initial_language_from_url_to_localstorage: bool,
    pub initial_language_from_localstorage: bool,
    pub initial_language_from_navigator: bool,
    pub localstorage_key: &'static str,
}

impl Clone for I18n {
    fn clone(&self) -> Self {
        Self {
            language: Rc::clone(&self.language),
            languages: self.languages,
            locales: self.locales,
            initial_language_from_url: self.initial_language_from_url,
            initial_language_from_url_param: self
                .initial_language_from_url_param,
            initial_language_from_url_to_localstorage: self
                .initial_language_from_url_to_localstorage,
            initial_language_from_localstorage: self
                .initial_language_from_localstorage,
            initial_language_from_navigator: self
                .initial_language_from_navigator,
            localstorage_key: self.localstorage_key,
        }
    }
}

impl I18n {
    /// Provides to Leptos the internationalization context
    pub fn provide_context(&self, initial_language: Option<&'static Language>) {
        if let Some(lang) = initial_language {
            self.language.set(lang);
        } else {
            let mut lang: Option<&'static Language> = None;
            if self.initial_language_from_url {
                if let Some(l) = url::get(self.initial_language_from_url_param)
                {
                    lang = self.language_from_str(&l);
                    if let Some(l) = lang {
                        if self.initial_language_from_url_to_localstorage {
                            localstorage::set(
                                self.localstorage_key,
                                &l.id.to_string(),
                            );
                        }
                    }
                }
            }

            if self.initial_language_from_localstorage && lang.is_none() {
                if let Some(l) = localstorage::get(self.localstorage_key) {
                    lang = self.language_from_str(&l);
                }
            }

            if self.initial_language_from_navigator && lang.is_none() {
                let languages = window().navigator().languages().to_vec();
                for raw_language in languages {
                    let language = raw_language.as_string();
                    if language.is_none() {
                        continue;
                    }
                    if let Some(l) = self.language_from_str(&language.unwrap())
                    {
                        lang = Some(l);
                        break;
                    }
                }
            }

            if let Some(l) = lang {
                self.language.set(l);
            }
        }
        provide_context::<I18n>(self.clone());
    }

    /// Translate a text identifier to the current language
    pub fn tr(&self, text_id: &str) -> String {
        let lang_id = &self.language.get().id;
        self.locales.lookup(lang_id, text_id).unwrap_or_else(|| {
            panic!(
                "Translation for '{}' not found in locale '{}'",
                text_id, lang_id
            )
        })
    }

    /// Translate a text identifier to the current language with arguments
    pub fn trs(
        &self,
        text_id: &str,
        args: &HashMap<String, FluentValue<'_>>,
    ) -> String {
        let lang_id = &self.language.get().id;
        self.locales
            .lookup_with_args(lang_id, text_id, args)
            .unwrap_or_else(|| {
                panic!(
                    "Translation for '{}' not found in locale '{}'",
                    text_id, lang_id
                )
            })
    }

    /// Get the default language
    ///
    /// The default language is the first language in the list of available languages.
    pub fn default_language(&self) -> &'static Language {
        self.languages[0]
    }

    /// Get the language from a language identifier
    ///
    /// This function will try to match the language identifier with the available
    /// languages. If it doesn't find an exact match, it will try to match the
    /// language identifier without the region. If it doesn't find a match, it will
    /// return `None`.
    pub fn language_from_str(&self, code: &str) -> Option<&'static Language> {
        match LanguageIdentifier::from_str(code) {
            Ok(target_lang) => match self
                .languages
                .iter()
                .find(|lang| lang.id.matches(&target_lang, false, false))
            {
                Some(lang) => Some(lang),
                None => {
                    let mut lazy_target_lang = target_lang.clone();
                    lazy_target_lang.region = None;
                    match self.languages.iter().find(|lang| {
                        lang.id.matches(&lazy_target_lang, true, true)
                    }) {
                        Some(lang) => Some(lang),
                        None => None,
                    }
                }
            },
            Err(_) => None,
        }
    }

    /// Set the current language in the signal of the context and in local storage
    pub fn set_language_with_localstorage(&self, lang: &'static Language) {
        self.language.set(lang);
        localstorage::set(self.localstorage_key, &lang.id.to_string());
    }
}

/// Get the current context for internationalization
pub fn i18n() -> I18n {
    expect_context::<I18n>()
}
