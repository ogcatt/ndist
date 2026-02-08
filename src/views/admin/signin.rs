use dioxus::prelude::*;
use crate::components::account_popup::AccountPopupContent;

#[component]
pub fn SignIn() -> Element {
    rsx! {
        div { class: "min-h-screen flex items-center justify-center bg-white",
            div { class: "max-w-md w-full space-y-8 p-8 bg-white rounded-lg border-gray-200 border",
                div { class: "text-center",
                    h2 { class: "text-2xl",
                        "Admin Sign In"
                    }
                    p { class: "mt-2 text-gray-600",
                        "Sign in with your email and OTP code."
                    }
                }
            }
            // Render the AccountPopup content directly in this page
            AccountPopupContent {
                email: use_signal(|| String::new()),
                on_close: move || {
                    // In admin context, just reload to go back to home
                    web_sys::window()
                        .unwrap()
                        .location()
                        .set_href("/")
                        .unwrap();
                },
            }
        }
    }
}