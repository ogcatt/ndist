#![allow(non_snake_case)]
use dioxus::prelude::*;
use crate::Route;

// Create a global signal for account popup state
pub fn use_account_popup() -> Option<Signal<bool>> {
    Some(use_context::<Signal<bool>>())
}

#[component]
pub fn AccountPopupProvider(children: Element) -> Element {
    let mut account_popup_open = use_signal(|| false);
    let mut email = use_signal(|| String::new());

    // Provide the signal to children
    use_context_provider(|| account_popup_open);

    rsx! {
        {children}
        if *account_popup_open.read() {
            div {
                class: "fixed inset-0 bg-black bg-opacity-50 z-[9999] flex items-center justify-center p-4",
                onclick: move |_| account_popup_open.set(false),
                div {
                    class: "bg-white rounded-lg shadow-xl max-w-md w-full",
                    onclick: move |e| e.stop_propagation(),
                    div {
                        class: "p-6",
                        div {
                            class: "flex justify-center mb-4",
                            img {
                                src: asset!("/assets/icons/person-circle-outline.svg"),
                                style: "height: 64px;"
                            }
                        },
                        div {
                            class: "text-center mb-6",
                            h2 {
                                class: "text-xl font-bold text-gray-900",
                                "Sign in or create account"
                            }
                        },
                        div {
                            class: "space-y-4",
                            input {
                                class: "w-full px-4 py-3 text-base border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500",
                                r#type: "email",
                                placeholder: "Email address",
                                value: "{email.read()}",
                                oninput: move |evt| {
                                    email.set(evt.value());
                                }
                            },
                            button {
                                class: "w-full bg-black text-white py-3 px-4 rounded-md font-medium hover:bg-gray-800 transition-colors",
                                "Continue"
                            }
                        },
                        div {
                            class: "mt-6 text-center",
                            p {
                                class: "text-sm text-gray-600",
                                "By continuing, you agree to our ",
                                Link {
                                    to: Route::Policies {},
                                    class: "text-blue-600 hover:underline",
                                    "Terms & Privacy Policy"
                                },
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn AccountButton() -> Element {
    let Some(mut account_popup_open) = use_account_popup() else {
        return rsx! { button { class: "h-full", title: "Account", div { class: "flex justify-center", img { class: "fadey", src: asset!("/assets/icons/person-circle-outline.svg"), style: "height:27px;" } } } };
    };

    rsx! {
        button {
            class: "h-full",
            title: "Account",
            onclick: move |_| {
                account_popup_open.set(true);
            },
            div {
                class: "flex justify-center",
                img {
                    class: "fadey",
                    src: asset!("/assets/icons/person-circle-outline.svg"),
                    style: "height:27px;"
                }
            }
        }
    }
}

#[component]
pub fn AccountMobileButton() -> Element {
    let Some(mut account_popup_open) = use_account_popup() else {
        return rsx! { button { class: "w-full px-4 py-3 flex items-center text-gray-900", img { src: asset!("/assets/icons/person-circle-outline.svg") }, span { "Sign in" } } };
    };

    rsx! {
        button {
            onclick: move |_| {
                account_popup_open.set(true);
            },
            class: "w-full px-4 py-3 flex items-center text-gray-900 hover:bg-gray-100 transition-colors duration-200 ease-out",
            img {
                class: "blende mr-3",
                src: asset!("/assets/icons/person-circle-outline.svg"),
                style: "height:20px;"
            },
            span {
                class: "text-sm font-semibold flex-1 text-left",
                "Sign in or create account"
            }
        }
    }
}
