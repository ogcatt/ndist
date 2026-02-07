use dioxus::prelude::*;
use crate::backend::server_functions::{send_magic_link, AuthResponse};

#[component]
pub fn SignIn() -> Element {
    let mut email = use_signal(String::new);
    let mut message = use_signal(String::new);
    let mut loading = use_signal(|| false);

    let handle_submit = move |_| {
        if !email.read().is_empty() && !loading() {
            loading.set(true);
            message.set(String::new()); // Clear previous message
            let email_val = email.read().clone();
            
            // Spawn the async task manually
            spawn(async move {
                let response = send_magic_link(email_val).await.unwrap_or_else(|e| AuthResponse {
                    success: false,
                    message: format!("Error: {}", e),
                });
                
                loading.set(false);
                message.set(response.message);
            });
        }
    };

    rsx! {
        div { class: "min-h-screen flex items-center justify-center bg-white",
            div { class: "max-w-md w-full space-y-8 p-8 bg-white rounded-lg border-gray-200 border",
            //rounded-lg relative overflow-hidden p-4 bg-ui-bg-subtle shadow-elevation-card-rest rounded-large group-hover:shadow-elevation-card-hover transition-shadow ease-in-out duration-150 aspect-[1/1] w-full
                div { class: "text-center",
                    h2 { class: "text-2xl",
                        "Manager Sign In"
                    }
                    p { class: "mt-2 text-gray-600",
                        "Enter your email to receive a sign in link."
                    }
                }
                
                form { class: "mt-8 space-y-6", onsubmit: handle_submit,
                    div {
                        label { class: "block text-sm font-medium text-gray-700",
                            "Email Address"
                        }
                        input {
                            r#type: "email",
                            required: true,
                            class: "mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500",
                            placeholder: "admin@example.com",
                            value: "{email}",
                            oninput: move |e| email.set(e.value())
                        }
                    }
                    
                    if !message.read().is_empty() {
                        div { 
                            class: format!("text-sm text-center text-green-600"),
                            "{message}"
                        }
                    }
                    
                    button {
                        r#type: "submit",
                        disabled: loading(),
                        class: "w-full flex justify-center py-2 px-4 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50",
                        if loading() {
                            "Sending..."
                        } else {
                            "Send Link"
                        }
                    }
                }
            }
        }
    }
}