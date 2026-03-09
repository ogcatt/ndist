#![allow(non_snake_case)]

use crate::backend::server_functions::{get_store_settings, admin_update_store_settings};
use crate::components::*;
use dioxus::prelude::*;

#[component]
pub fn AdminSettings() -> Element {
    let settings_res = use_resource(move || async move { get_store_settings().await });

    let mut lock_store = use_signal(|| false);
    let mut lock_comment = use_signal(|| String::new());
    let mut feedback = use_signal(|| Option::<(bool, String)>::None);
    let mut saving = use_signal(|| false);

    // Load settings into signals when resource resolves
    use_effect(move || {
        if let Some(Ok(s)) = settings_res.read().as_ref() {
            lock_store.set(s.lock_store);
            lock_comment.set(s.lock_comment.clone().unwrap_or_default());
        }
    });

    let on_save = move |_| {
        let ls = lock_store();
        let lc = lock_comment().clone();
        async move {
            saving.set(true);
            feedback.set(None);
            match admin_update_store_settings(ls, lc).await {
                Ok(_) => feedback.set(Some((true, "Settings saved.".to_string()))),
                Err(e) => feedback.set(Some((false, format!("Error: {}", e)))),
            }
            saving.set(false);
        }
    };

    rsx! {
        div {
            class: "w-full",

            // Header
            div {
                class: "bg-white border rounded-md border-gray-200 p-4 mb-4 h-20 flex justify-between items-center",
                div {
                    class: "text-lg font-medium",
                    "Store Settings"
                }
            }

            // Settings card
            div {
                class: "bg-white border rounded-md border-gray-200 p-6 max-w-xl",

                div {
                    class: "text-sm font-semibold text-gray-700 mb-4",
                    "Store Lock"
                }

                div {
                    class: "flex items-center justify-between mb-4",
                    div {
                        div { class: "text-sm font-medium text-gray-800", "Lock Store" }
                        div { class: "text-xs text-gray-500", "Overlays the store with an unavailability message for non-admin users." }
                    }
                    CToggle {
                        checked: lock_store(),
                        onclick: move |_| lock_store.toggle(),
                    }
                }

                div {
                    class: "mb-6",
                    label {
                        class: "block text-sm font-medium text-gray-700 mb-1",
                        "Lock Comment"
                    }
                    div { class: "text-xs text-gray-500 mb-2", "Optional message shown to users when the store is locked." }
                    CTextArea {
                        value: lock_comment(),
                        placeholder: "e.g. We'll be back soon!",
                        oninput: move |e: FormEvent| lock_comment.set(e.value()),
                        rows: 3,
                    }
                }

                // Feedback
                if let Some((ok, msg)) = feedback() {
                    div {
                        class: if ok { "text-sm text-green-600 mb-3" } else { "text-sm text-red-600 mb-3" },
                        "{msg}"
                    }
                }

                button {
                    class: "bg-zinc-600 text-white text-sm px-4 py-2 rounded hover:bg-zinc-500 transition-colors disabled:opacity-50",
                    disabled: saving(),
                    onclick: on_save,
                    if saving() { "Saving..." } else { "Save Settings" }
                }
            }

            // API Keys informational card
            div {
                class: "bg-white border rounded-md border-gray-200 p-6 max-w-xl mt-4",

                div {
                    class: "text-sm font-semibold text-gray-700 mb-3",
                    "API Keys"
                }

                p { class: "text-sm text-gray-600 mb-3",
                    "API keys are managed per-group and allow external services to generate invite codes programmatically."
                }

                p { class: "text-sm text-gray-500 mb-4",
                    "Go to a group's edit page to create or manage its API keys."
                }

                a {
                    href: "/admin/dashboard/groups",
                    class: "inline-flex items-center text-sm font-medium text-blue-600 hover:text-blue-800",
                    "Manage Groups"
                    span { class: "ml-1", " →" }
                }
            }
        }
    }
}
