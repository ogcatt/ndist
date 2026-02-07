use dioxus::prelude::*;
use web_sys::window;
use crate::backend::server_functions::*;
use url::Url;

#[component]
pub fn VerifyMagicLink() -> Element {
    let mut verification_status = use_signal(|| "verifying".to_string());
    let navigator = use_navigator();

    // Extract token from URL on mount
    use_effect(move || {
        spawn(async move {
            #[cfg(target_arch = "wasm32")]
            {
                if let Some(window) = window() {
                    if let Ok(url) = window.location().href() {
                        if let Ok(parsed_url) = Url::parse(&url) {
                            // Extract access_token from URL fragment or query params
                            let fragment = parsed_url.fragment().unwrap_or("");
                            let params: std::collections::HashMap<_, _> = fragment
                                .split('&')
                                .filter_map(|param| {
                                    let mut parts = param.split('=');
                                    match (parts.next(), parts.next()) {
                                        (Some(key), Some(value)) => Some((key, value)),
                                        _ => None,
                                    }
                                })
                                .collect();

                            if let Some(access_token) = params.get("access_token") {
                                match verify_magic_link(access_token.to_string()).await {
                                    Ok(response) => {
                                        if response.success {
                                            verification_status.set("success".to_string());
                                            // Redirect to dashboard after 2 seconds
                                            //tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                                            navigator.push("/admin/dashboard");
                                        } else {
                                            verification_status.set(format!("error: {}", response.message));
                                        }
                                    }
                                    Err(e) => {
                                        verification_status.set(format!("error: {}", e));
                                    }
                                }
                            } else {
                                verification_status.set("error: No access token found".to_string());
                            }
                        }
                    }
                }
            }
        });
    });

    rsx! {
        div { class: "min-h-screen flex items-center justify-center bg-white",
            div { class: "max-w-md w-full space-y-8 p-8 bg-white rounded-lg border-gray-200 border text-center",
                {
                    let status = verification_status();
                    match status.as_str() {
                        "verifying" => rsx! {
                            div { class: "animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 mx-auto" }
                            h2 { class: "mt-4 text-xl text-black",
                                "Verifying your magic link..."
                            }
                        },
                        "success" => rsx! {
                            div { class: "text-green-600",
                                svg { class: "w-12 h-12 mx-auto", fill: "currentColor", view_box: "0 0 20 20",
                                    path { fill_rule: "evenodd", d: "M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z", clip_rule: "evenodd" }
                                }
                            }
                            h2 { class: "mt-4 text-xl font-semibold text-gray-900",
                                "Successfully authenticated!"
                            }
                            p { class: "mt-2 text-gray-600",
                                "Redirecting to dashboard..."
                            }
                        },
                        _ => {
                            let error_msg = status.strip_prefix("error: ").unwrap_or(&status);
                            rsx! {
                                div { class: "text-red-600",
                                    svg { class: "w-12 h-12 mx-auto", fill: "currentColor", view_box: "0 0 20 20",
                                        path { fill_rule: "evenodd", d: "M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z", clip_rule: "evenodd" }
                                    }
                                }
                                h2 { class: "mt-4 text-xl font-semibold text-gray-900",
                                    "Authentication failed"
                                }
                                p { class: "mt-2 text-gray-600",
                                    "{error_msg}"
                                }
                                a { href: "/admin/signin", class: "mt-4 inline-block px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700",
                                    "Try again"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}