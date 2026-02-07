#![allow(non_snake_case)]
use crate::i18n::{Language, use_language_setter};
use crate::utils::countries::*;
use dioxus::prelude::*;
use dioxus_i18n::{prelude::*, t};
use std::str::FromStr;

#[component]
pub fn LanguagePopup() -> Element {
    let mut show_popup = crate::i18n::use_language_popup();
    let mut language_setter = use_language_setter();
    let mut handle_language_select = move |lang_code: &str| {
        if let Ok(language) = Language::from_str(lang_code) {
            // Save to storage first
            if let Err(e) = crate::i18n::set_user_language(language.clone()) {
                tracing::error!("Failed to save language: {}", e);
            }
            // Update the signal
            language_setter.set(Some(language));
            // Hide popup
            show_popup.set(false);
            tracing::info!("User selected language: {}", lang_code);
        }
    };

    if !*show_popup.read() {
        return rsx! { div {} };
    }

    rsx! {
        div {
            class: "fixed inset-0 bg-black bg-opacity-50 z-[9999] flex items-center justify-center p-4",
            onclick: move |_| show_popup.set(false),
            div {
                class: "bg-white rounded-lg shadow-xl max-w-md w-full max-h-[90vh] flex flex-col",
                onclick: move |e| e.stop_propagation(),
                div {
                    class: "text-center p-6 pb-4 flex-shrink-0",
                    h2 {
                        class: "text-2xl font-bold text-gray-900 mb-2",
                        "Select Your Language"
                    },
                    p {
                        class: "text-gray-600",
                        "Choose your preferred language for the best experience"
                    }
                },
                div {
                    class: "flex-1 overflow-y-auto px-6",
                    div {
                        class: "space-y-3",
                        for language_option in LANGUAGE_OPTIONS.iter() {
                            button {
                                class: "w-full flex items-center justify-between p-4 border-2 border-gray-200 rounded-lg hover:border-blue-500 hover:bg-blue-50 transition-colors",
                                onclick: move |_| handle_language_select(language_option.code),
                                div {
                                    class: "flex items-center",
                                    div {
                                        class: "text-2xl mr-3",
                                        "{language_option.flag}"
                                    },
                                    div {
                                        class: "text-left",
                                        div {
                                            class: "font-semibold text-gray-900",
                                            "{language_option.name}"
                                        },
                                        div {
                                            class: "text-sm text-gray-500",
                                            "{language_option.country}"
                                        }
                                    }
                                },
                                div {
                                    class: "text-gray-400",
                                    "→"
                                }
                            }
                        }
                    }
                },
                div {
                    class: "p-6 pt-4 text-center flex-shrink-0",
                    p {
                        class: "text-xs text-gray-500",
                        "You can change this later in the navigation menu. By continuing you permit us to store cookies for basic site functionality."
                    }
                }
            }
        }
    }
}
