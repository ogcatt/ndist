use dioxus::prelude::*;
use crate::components::account_popup::{AccountPopupContent, SessionState};

#[component]
pub fn SignIn() -> Element {
    let session_state = use_signal(|| SessionState {
        authenticated: false,
        email: String::new(),
        name: String::new(),
        admin: false,
    });

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
                session_state: session_state,
                on_close: move |_| {
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