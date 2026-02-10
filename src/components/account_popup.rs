#![allow(non_snake_case)]
use dioxus::prelude::*;
use crate::Route;
use crate::backend::server_functions::{send_otp, verify_otp, VerifyOtpResponse, get_session_info};
use std::rc::Rc;
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use gloo_timers::future::TimeoutFuture;

// Session info for UI state
#[derive(Clone, Debug, Default)]
pub struct SessionState {
    pub authenticated: bool,
    pub email: String,
    pub name: String,
    pub admin: bool,
}

// Global storage for session state (use Rc<RefCell<...>> for interior mutability)
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use once_cell::sync::Lazy;

// Using atomic globals to avoid context issues during hydration
static SESSION_STORAGE: Lazy<Mutex<Option<SessionState>>> = Lazy::new(|| Mutex::new(None));
static POPUP_OPEN: AtomicBool = AtomicBool::new(false);

// Hook to get session state - safe for SSR and hydration
fn use_global_session_state() -> SessionState {
    SESSION_STORAGE.lock().unwrap().clone().unwrap_or_default()
}

// Hook to set session state
fn set_global_session_state(state: SessionState) {
    *SESSION_STORAGE.lock().unwrap() = Some(state);
}

// Create a global signal for account popup state (uses atomics for safety)
pub fn use_account_popup_open() -> bool {
    POPUP_OPEN.load(Ordering::Relaxed)
}

pub fn set_account_popup_open(open: bool) {
    POPUP_OPEN.store(open, Ordering::Relaxed);
}

pub fn use_account_popup() -> Option<Signal<bool>> {
    Some(use_context::<Signal<bool>>())
}

#[component]
pub fn AccountPopupProvider(children: Element) -> Element {
    let mut account_popup_open = use_signal(|| false);
    let mut email = use_signal(|| String::new());
    let mut session_state = use_signal(|| SessionState {
        authenticated: false,
        email: String::new(),
        name: String::new(),
        admin: false,
    });

    // Check session on mount
    use_effect(move || {
        spawn(async move {
            match get_session_info().await {
                Ok(session) => {
                    let state = SessionState {
                        authenticated: session.authenticated,
                        email: session.email,
                        name: session.name,
                        admin: session.admin,
                    };
                    session_state.set(state.clone());
                    set_global_session_state(state);
                }
                Err(_) => {
                    let state = SessionState {
                        authenticated: false,
                        email: String::new(),
                        name: String::new(),
                        admin: false,
                    };
                    session_state.set(state.clone());
                    set_global_session_state(state);
                }
            }
        });
    });

    // Sync popup state with global
    let popup_open_global = use_global_session_state;
    use_effect(move || {
        let open = *account_popup_open.read();
        set_account_popup_open(open);
    });

    // Provide the signal to children (for components that still use context)
    use_context_provider(|| account_popup_open);
    use_context_provider(|| session_state);

    rsx! {
        {children}
        if *account_popup_open.read() {
            AccountPopupContent {
                email: email,
                session_state: session_state,
                on_close: move || account_popup_open.set(false),
            }
        }
    }
}

#[component]
pub fn AccountPopupContent(
    email: Signal<String>,
    session_state: Signal<SessionState>,
    on_close: EventHandler<()>,
) -> Element {
    let mut step = use_signal(|| 0i32); // 0: email input, 1: OTP input
    let mut email_val = use_signal(|| email.read().clone());
    let mut otp_code = use_signal(|| vec![String::new(); 6]);
    let mut message = use_signal(|| String::new());
    let mut loading = use_signal(|| false);
    let mut focused_input = use_signal(|| 0usize);
    let mut input_refs = use_signal(|| Vec::<Rc<MountedData>>::new());
    let mut cooldown_seconds = use_signal(|| 0i32);

    // Focus effect for OTP inputs
    use_effect(move || {
        let focused_index = focused_input();
        let refs = input_refs.peek();
        let ref_data = refs.get(focused_index).cloned();
        if let Some(data) = ref_data {
            spawn(async move {
                let _ = data.set_focus(true).await;
            });
        }
    });

    let is_otp_complete = use_memo(move || otp_code().iter().all(|s| !s.is_empty()));

    // Determine button text based on state
    let button_text = use_memo(move || {
        if loading() {
            "Sending...".to_string()
        } else if cooldown_seconds() > 0 {
            format!("Wait {} seconds", cooldown_seconds())
        } else {
            "Send Code".to_string()
        }
    });

    // Start cooldown timer using gloo timers
    let mut start_cooldown = move |secs: i32| {
        if secs <= 0 { return; }
        cooldown_seconds.set(secs);
        let mut remaining = secs;
        
        spawn(async move {
            while remaining > 0 {
                TimeoutFuture::new(1000).await;
                remaining -= 1;
                cooldown_seconds.set(remaining);
            }
        });
    };

    let handle_send_otp = move |_| {
        if email_val.read().is_empty() || loading() || cooldown_seconds() > 0 {
            return;
        }
        loading.set(true);
        message.set(String::new());

        let email_input = email_val.read().clone();
        spawn(async move {
            let response = send_otp(email_input).await;
            loading.set(false);
            match response {
                Ok(res) => {
                    if res.success {
                        step.set(1);
                        message.set(String::new());
                        // Reset OTP inputs and refs when moving to OTP step
                        otp_code.set(vec![String::new(); 6]);
                        input_refs.set(Vec::new());
                        focused_input.set(0);
                    } else {
                        message.set(res.message.clone());
                        // Check if it's a rate limit message and extract cooldown time
                        if res.message.contains("wait") {
                            // Parse the remaining seconds from the message
                            let nums: String = res.message
                                .chars()
                                .filter(|c| c.is_ascii_digit())
                                .collect();
                            if let Ok(secs) = nums.parse::<i32>() {
                                if secs > 0 {
                                    start_cooldown(secs);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    message.set(format!("Error: {}", e));
                }
            }
        });
    };

    let handle_verify_otp = move |_| {
        if !is_otp_complete() || loading() {
            return;
        }
        loading.set(true);
        message.set(String::new());

        let email_input = email_val.read().clone();
        let otp_input = otp_code.read().clone().join("");
        spawn(async move {
            let response = verify_otp(email_input, otp_input).await;
            loading.set(false);
            match response {
                Ok(res) => {
                    if let Some(msg) = res.message {
                        // Show the error message from backend
                        message.set(msg);
                    } else if let Some(_token) = res.session_token {
                        // Success - close popup and redirect to dashboard
                        message.set("Sign in successful!".to_string());
                        web_sys::window()
                            .unwrap()
                            .location()
                            .set_href("/dashboard")
                            .unwrap();
                    } else if res.is_new_user {
                        // New user created - redirect to dashboard
                        message.set("Account created successfully!".to_string());
                        web_sys::window()
                            .unwrap()
                            .location()
                            .set_href("/dashboard")
                            .unwrap();
                    }
                }
                Err(e) => {
                    message.set(format!("Error: {}", e));
                }
            }
        });
    };

    // OTP Input Handlers
    let mut handle_code_input = move |index: usize, value: String| {
        let mut code = otp_code();
        let chars: Vec<char> = value.chars().collect();

        if value.len() > 1 && index == 0 {
            // Handle paste into first field
            let digits: Vec<String> = chars
                .iter()
                .filter(|c| c.is_ascii_digit())
                .take(6)
                .map(|c| c.to_string())
                .collect();
            for (i, digit) in digits.iter().enumerate() {
                if i < 6 {
                    code[i] = digit.clone();
                }
            }
            otp_code.set(code);
            let next_index = digits.len().min(5);
            focused_input.set(next_index);
        } else if chars.len() == 1 && chars[0].is_ascii_digit() {
            code[index] = value;
            otp_code.set(code);
            if index < 5 {
                let next_index = index + 1;
                focused_input.set(next_index);
            }
        } else if value.is_empty() {
            code[index] = String::new();
            otp_code.set(code);
            if index > 0 {
                let prev_index = index - 1;
                focused_input.set(prev_index);
            }
        }
    };

    let mut handle_keydown = move |index: usize, evt: Event<KeyboardData>| {
        if evt.key() == Key::Backspace && otp_code()[index].is_empty() && index > 0 {
            focused_input.set(index - 1);
        }
    };

    let handle_go_back = move |_| {
        step.set(0);
        otp_code.set(vec![String::new(); 6]);
        message.set(String::new());
        focused_input.set(0);
        input_refs.set(Vec::new());
        cooldown_seconds.set(0);
    };

    rsx! {
        div {
            class: "fixed inset-0 bg-black bg-opacity-50 z-[9999] flex items-center justify-center p-4",
            onclick: move |_| on_close(()),
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
                        },
                    },
                    div {
                        class: "text-center mb-6",
                        h2 {
                            class: "text-2xl font-bold text-gray-900",
                            if step() == 0 {
                                "Sign in"
                            } else {
                                "Enter Login Code"
                            }
                        }
                        if step() == 1 {
                            p {
                                class: "text-sm text-gray-500 mt-2",
                                "Please enter the 6-digit code sent to your email"
                            }
                        }
                    },
                    if step() == 0 {
                        // Email input step
                        div {
                            class: "space-y-4",
                            input {
                                class: "w-full px-4 py-3 text-base border-2 border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500 transition-colors duration-200",
                                r#type: "email",
                                placeholder: "Email address",
                                value: "{email_val.read()}",
                                oninput: move |evt| email_val.set(evt.value())
                            },
                            button {
                                class: "w-full bg-blue-600 text-white py-3 px-4 rounded-lg font-medium hover:bg-blue-700 transition-colors duration-200 disabled:opacity-50 disabled:cursor-not-allowed focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2",
                                disabled: loading() || cooldown_seconds() > 0,
                                onclick: handle_send_otp,
                                "{button_text}"
                            }
                        }
                    } else {
                        // OTP verification step
                        div {
                            class: "space-y-6",
                            div {
                                class: "flex justify-center space-x-3",
                                {(0..6).map(|i| {
                                    let code_value = otp_code()[i].clone();
                                    rsx! {
                                        input {
                                            key: "{i}",
                                            class: "w-12 h-16 text-center text-2xl font-bold border-2 border-gray-300 rounded-lg focus:border-blue-500 focus:outline-none transition-colors duration-200",
                                            r#type: "text",
                                            maxlength: if i == 0 { "6" } else { "1" },
                                            value: "{code_value}",
                                            onmounted: move |cx| input_refs.write().push(cx.data()),
                                            oninput: move |evt| handle_code_input(i, evt.value()),
                                            onkeydown: move |evt| handle_keydown(i, evt),
                                        }
                                    }
                                })}
                            }
                            button {
                                class: "w-full bg-blue-600 text-white py-3 px-4 rounded-lg font-medium hover:bg-blue-700 transition-colors duration-200 disabled:opacity-50 disabled:cursor-not-allowed focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2",
                                disabled: !is_otp_complete() || loading(),
                                onclick: handle_verify_otp,
                                if loading() {
                                    "Verifying..."
                                } else {
                                    "Verify Code"
                                }
                            },
                            div {
                                class: "text-center",
                                a {
                                    class: "text-xs text-gray-500 hover:text-gray-700 cursor-pointer",
                                    onclick: handle_go_back,
                                    "Go back"
                                }
                            }
                        }
                    },
                    if !message.read().is_empty() {
                        div {
                            class: format!("mt-4 text-sm text-center {}",
                                if message.read().contains("successful") || message.read().contains("OTP sent") || message.read().contains("Account created") {
                                    "text-green-600"
                                } else {
                                    "text-red-600"
                                }
                            ),
                            "{message}"
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

// Get session state from global storage - safe for SSR and hydration
fn use_session_state() -> SessionState {
    use_global_session_state()
}

#[component]
pub fn AccountButton() -> Element {
    let mut session_resource = use_resource(move || async move {
        get_session_info().await
    });

    let Some(mut account_popup_open) = use_account_popup() else {
        // Return placeholder button when no context available
        return rsx! {
            button {
                class: "h-full",
                title: "Account",
                div {
                    class: "flex justify-center",
                    img {
                        class: "fadey",
                        src: asset!("/assets/icons/person-circle-outline.svg"),
                        style: "height:27px;"
                    }
                }
            }
        };
    };

    let session_state = session_resource.read();
    let is_authenticated = session_state
        .as_ref()
        .map(|s| s.as_ref().map(|session| session.authenticated).unwrap_or(false))
        .unwrap_or(false);

    if is_authenticated {
        rsx! {
            Link {
                to: Route::UserDashboard {},
                class: "h-full",
                title: "Dashboard",
                button {
                    class: "h-full",
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
    } else {
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
}

#[component]
pub fn AccountMobileButton() -> Element {
    let mut session_resource = use_resource(move || async move {
        get_session_info().await
    });

    let Some(mut account_popup_open) = use_account_popup() else {
        // Return placeholder link when no context available
        return rsx! {
            Link {
                to: Route::UserDashboard {},
                class: "w-full px-4 py-3 flex items-center text-gray-900",
                img {
                    class: "blende mr-3",
                    src: asset!("/assets/icons/person-circle-outline.svg"),
                    style: "height:20px;"
                },
                span {
                    class: "text-sm font-semibold flex-1 text-left",
                    "Dashboard"
                }
            }
        };
    };

    let session_state = session_resource.read();
    let is_authenticated = session_state
        .as_ref()
        .map(|s| s.as_ref().map(|session| session.authenticated).unwrap_or(false))
        .unwrap_or(false);

    if is_authenticated {
        rsx! {
            Link {
                to: Route::UserDashboard {},
                class: "w-full px-4 py-3 flex items-center text-gray-900 hover:bg-gray-100 transition-colors duration-200 ease-out",
                img {
                    class: "blende mr-3",
                    src: asset!("/assets/icons/person-circle-outline.svg"),
                    style: "height:20px;"
                },
                span {
                    class: "text-sm font-semibold flex-1 text-left",
                    "Dashboard"
                }
            }
        }
    } else {
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
}