use dioxus::prelude::*;
use crate::backend::server_functions::get_session_info;

#[component]
pub fn UserDashboard() -> Element {
    let mut session_resource = use_resource(move || async move {
        get_session_info().await
    });

    let session_state = session_resource.read();

    // Redirect to home if not authenticated
    if let Some(Ok(session)) = session_state.as_ref() {
        if !session.authenticated {
            use_effect(move || {
                let _ = web_sys::window()
                    .unwrap()
                    .location()
                    .set_href("/");
            });
        }
    } else if let Some(Err(_)) = session_state.as_ref() {
        use_effect(move || {
            let _ = web_sys::window()
                .unwrap()
                .location()
                .set_href("/");
        });
    }

    rsx! {
        div { class: "pt-8 md:pt-12",
            div { class: "content-container py-0 sm:pt-0 sm:pb-12 px-6",
                div { class: "md:flex md:items-center md:justify-between mb-8",
                    div { class: "flex-1 min-w-0",
                        h2 { class: "text-2xl font-bold leading-7 text-gray-900 sm:text-3xl sm:truncate",
                            "Dashboard" }
                    }
                }
                if let Some(Ok(session)) = session_state.as_ref() {
                    if session.authenticated {
                        div { class: "bg-white shadow overflow-hidden sm:rounded-lg",
                            div { class: "px-4 py-5 sm:px-6",
                                h3 { class: "text-lg leading-6 font-medium text-gray-900",
                                    "Account Overview" }
                                p { class: "mt-1 max-w-2xl text-sm text-gray-500",
                                    "Your account information and settings" }
                            }
                            div { class: "border-t border-gray-200 px-4 py-5 sm:p-0",
                                dl { class: "sm:divide-y sm:divide-gray-200",
                                    div { class: "py-4 sm:py-5 sm:grid sm:grid-cols-3 sm:gap-4 sm:px-6",
                                        dt { class: "text-sm font-medium text-gray-500",
                                            "Email" }
                                        dd { class: "mt-1 text-sm text-gray-900 sm:mt-0 sm:col-span-2",
                                            "{session.email}" }
                                    }
                                    if !session.name.is_empty() && session.name != session.email {
                                        div { class: "py-4 sm:py-5 sm:grid sm:grid-cols-3 sm:gap-4 sm:px-6",

                                            dt { class: "text-sm font-medium text-gray-500",
                                                "Name"
                                            }
                                            dd { class: "mt-1 text-sm text-gray-900 sm:mt-0 sm:col-span-2",
                                                "{session.name}"
                                            }

                                        }
                                    } else {
                                        // Hide name section entirely if empty or same as email
                                        ""
                                    }
                                    div { class: "py-4 sm:py-5 sm:grid sm:grid-cols-3 sm:gap-4 sm:px-6",
                                        dt { class: "text-sm font-medium text-gray-500",
                                            "Account Status" }
                                        dd { class: "mt-1 text-sm text-gray-900 sm:mt-0 sm:col-span-2",
                                            "Active" }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    div { class: "text-center py-12",
                        p { "Loading..." }
                    }
                }
            }
        }
    }
}
