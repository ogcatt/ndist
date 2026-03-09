use dioxus::prelude::*;
use dioxus_i18n::t;

use crate::backend::server_functions::*;
use crate::Route;

#[derive(Clone, Copy)]
struct AdminDarkMode(Signal<bool>);

#[component]
pub fn AdminWrapper() -> Element {
    let mut is_sidebar_open = use_signal(|| false);
    let mut is_user_menu_open = use_signal(|| false);
    let mut dark_mode = use_signal(|| true);

    use_context_provider(|| AdminDarkMode(dark_mode));

    let current_user = use_resource(|| get_current_user());

    // Load saved preference from localStorage
    use_effect(move || {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                if let Ok(Some(val)) = storage.get_item("admin_dark_mode") {
                    dark_mode.set(val != "false");
                }
            }
        }
    });

    let handle_logout = move |_| {
        spawn(async move {
            match logout_user().await {
                Ok(_) => {
                    web_sys::window()
                        .unwrap()
                        .location()
                        .reload()
                        .unwrap();
                }
                Err(e) => {
                    println!("Logout error: {}", e);
                }
            }
        });
    };

    let close_sidebar = move |_| {
        is_sidebar_open.set(false);
        is_user_menu_open.set(false);
    };

    let toggle_dark_mode = move |_| {
        let new_val = !dark_mode();
        dark_mode.set(new_val);
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                let _ = storage.set_item("admin_dark_mode", if new_val { "true" } else { "false" });
            }
        }
    };

    let dm = dark_mode();

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/assets/styling/admin.css") }
        document::Title { { format!("{} - Admin Panel", t!("brand") ) } }

        div {
            class: if dm { "admin-dark min-h-screen bg-zinc-900 font-mono" } else { "min-h-screen bg-zinc-50 font-mono" },

            if is_sidebar_open() {
                div {
                    class: "fixed inset-0 z-40 bg-black bg-opacity-50 lg:hidden",
                    onclick: close_sidebar,
                }
            }

            // Sidebar
            div {
                class: format!(
                    "fixed inset-y-0 left-0 z-50 w-64 {} transform transition-transform duration-200 ease-in-out lg:translate-x-0 {}",
                    if dm { "bg-zinc-800 border-r border-zinc-700" } else { "bg-white border-r border-zinc-200" },
                    if is_sidebar_open() { "translate-x-0" } else { "-translate-x-full" }
                ),

                div {
                    class: if dm { "flex items-center justify-between h-16 px-4 border-b border-zinc-700" } else { "flex items-center justify-between h-16 px-4 border-b border-zinc-200" },
                    Link {
                        to: Route::Home {},
                        class: "flex items-center hover:opacity-75 transition-opacity",
                        img {
                            src: asset!("/assets/images/header.avif"),
                            alt: t!("brand"),
                            class: if dm { "h-8 invert" } else { "h-8" }
                        }
                    }
                    button {
                        class: if dm { "lg:hidden p-1 text-zinc-400 hover:text-zinc-200" } else { "lg:hidden p-1 text-zinc-500 hover:text-zinc-700" },
                        onclick: move |_| is_sidebar_open.set(false),
                        svg { class: "w-5 h-5", fill: "none", stroke: "currentColor", view_box: "0 0 24 24",
                            path { stroke_linecap: "square", stroke_linejoin: "miter", stroke_width: "2", d: "M6 18L18 6M6 6l12 12" }
                        }
                    }
                }

                // Navigation
                nav {
                    class: "mt-4 px-2",
                    SidebarItem {
                        to: Route::AdminDashboard {},
                        icon: rsx! { img { class: "w-5 h-5", src: asset!("/assets/icons/home.svg") } },
                        label: "Overview",
                    }
                    SidebarItem {
                        to: Route::AdminOrders {},
                        icon: rsx! { img { class: "w-5 h-5", src: asset!("/assets/icons/file-tray-stacked.svg") } },
                        label: "Orders"
                    }
                    SidebarItem {
                        to: Route::AdminUsers {},
                        icon: rsx! { img { class: "w-5 h-5", src: asset!("/assets/icons/people.svg") } },
                        label: "Users"
                    }
                    SidebarItem {
                        to: Route::AdminGroups {},
                        icon: rsx! { img { class: "w-5 h-5", src: asset!("/assets/icons/people-circle-outline.svg") } },
                        label: "Groups"
                    }
                    SidebarItem {
                        to: Route::AdminProducts {},
                        icon: rsx! { img { class: "w-5 h-5", src: asset!("/assets/icons/inventory.svg") } },
                        label: "Products"
                    }
                    SidebarItem {
                        to: Route::AdminInventory {},
                        icon: rsx! { img { class: "w-5 h-5", src: asset!("/assets/icons/receipt.svg") } },
                        label: "Inventory"
                    }
                    SidebarItem {
                        to: Route::AdminDiscounts {},
                        icon: rsx! { img { class: "w-5 h-5", src: asset!("/assets/icons/pricetags.svg") } },
                        label: "Discounts"
                    }
                    SidebarItem {
                        to: Route::AdminContent {},
                        icon: rsx! { img { class: "w-5 h-5", src: asset!("/assets/icons/images.svg") } },
                        label: "Content"
                    }
                    SidebarItem {
                        to: Route::AdminAnalytics {},
                        icon: rsx! { img { class: "w-5 h-5", src: asset!("/assets/icons/bar-chart.svg") } },
                        label: "Analytics"
                    }

                    hr { class: if dm { "my-4 border-zinc-700" } else { "my-4 border-zinc-200" } }

                    SidebarItem {
                        to: Route::AdminSettings {},
                        icon: rsx! { img { class: "w-5 h-5", src: asset!("/assets/icons/settings.svg") } },
                        label: "Settings"
                    }
                }
            }

            // Main content
            div {
                class: "lg:pl-64",

                header {
                    class: if dm { "h-16 bg-zinc-800 border-b border-zinc-700 flex items-center justify-between px-4" } else { "h-16 bg-white border-b border-zinc-200 flex items-center justify-between px-4" },

                    button {
                        class: if dm { "lg:hidden p-2 text-zinc-400 hover:text-zinc-200 hover:bg-zinc-700 rounded-none transition-colors" } else { "lg:hidden p-2 text-zinc-500 hover:text-zinc-700 hover:bg-zinc-100 rounded-none transition-colors" },
                        onclick: move |_| is_sidebar_open.set(true),
                        svg { class: "w-5 h-5", fill: "none", stroke: "currentColor", view_box: "0 0 24 24",
                            path { stroke_linecap: "square", stroke_linejoin: "miter", stroke_width: "2", d: "M4 6h16M4 12h16M4 18h16" }
                        }
                    }

                    div { class: "flex-1" }

                    div {
                        class: "flex items-center space-x-2",

                        match current_user.read().as_ref() {
                            Some(Ok(Some(user))) => rsx! {
                                div {
                                    class: "relative",
                                    button {
                                        class: if dm { "flex items-center space-x-2 p-1 rounded-none hover:bg-zinc-700 transition-colors" } else { "flex items-center space-x-2 p-1 rounded-none hover:bg-zinc-100 transition-colors" },
                                        onclick: move |_| is_user_menu_open.set(!is_user_menu_open()),
                                        img {
                                            class: if dm { "w-8 h-8 invert" } else { "w-8 h-8" },
                                            src: asset!("/assets/icons/person-circle.svg")
                                        },
                                        span {
                                            class: if dm { "text-sm font-medium text-zinc-100 hidden sm:inline" } else { "text-sm font-medium text-zinc-900 hidden sm:inline" },
                                            "{user.name}"
                                        }
                                        svg {
                                            class: if dm { "w-4 h-4 text-zinc-400" } else { "w-4 h-4 text-zinc-500" },
                                            fill: "none", stroke: "currentColor", view_box: "0 0 24 24",
                                            path { stroke_linecap: "square", stroke_linejoin: "miter", stroke_width: "2", d: "M19 9l-7 7-7-7" }
                                        }
                                    }

                                    if is_user_menu_open() {
                                        div {
                                            class: if dm { "absolute right-0 mt-1 w-56 bg-zinc-800 border border-zinc-700 rounded-none shadow-lg z-50" } else { "absolute right-0 mt-1 w-56 bg-white border border-zinc-200 rounded-none shadow-lg z-50" },
                                            div {
                                                class: if dm { "p-3 border-b border-zinc-700" } else { "p-3 border-b border-zinc-200" },
                                                p { class: if dm { "text-sm font-medium text-zinc-100" } else { "text-sm font-medium text-zinc-900" }, "{user.name}" }
                                                p { class: if dm { "text-xs text-zinc-400" } else { "text-xs text-zinc-600" }, "{user.email}" }
                                            }
                                            div {
                                                class: "py-1",
                                                // Theme toggle
                                                button {
                                                    onclick: toggle_dark_mode,
                                                    class: if dm { "w-full text-left px-3 py-2 text-sm text-zinc-300 hover:bg-zinc-700 transition-colors flex items-center space-x-2" } else { "w-full text-left px-3 py-2 text-sm text-zinc-700 hover:bg-zinc-100 transition-colors flex items-center space-x-2" },
                                                    if dm {
                                                        svg { class: "w-4 h-4", fill: "none", stroke: "currentColor", view_box: "0 0 24 24",
                                                            path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2", d: "M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z" }
                                                        }
                                                        span { "Light mode" }
                                                    } else {
                                                        svg { class: "w-4 h-4", fill: "none", stroke: "currentColor", view_box: "0 0 24 24",
                                                            path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2", d: "M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z" }
                                                        }
                                                        span { "Dark mode" }
                                                    }
                                                }
                                                hr { class: if dm { "my-1 border-zinc-700" } else { "my-1 border-zinc-200" } }
                                                button {
                                                    onclick: handle_logout,
                                                    class: "w-full text-left px-3 py-2 text-sm text-red-500 hover:bg-red-900 hover:bg-opacity-20 transition-colors",
                                                    "Sign out"
                                                }
                                            }
                                        }
                                    }
                                }
                            },
                            None | Some(&Ok(None)) => rsx! {},
                            Some(Err(_)) => rsx! {
                                div { "Loading user info failed" }
                            },
                        }
                    }
                }

                main {
                    class: if dm { "min-h-screen p-3 md:p-4 bg-zinc-900" } else { "min-h-screen p-3 md:p-4 bg-zinc-100" },
                    Outlet::<Route> {}
                }
            }
        }
    }
}


#[component]
pub fn SidebarItem(icon: Element, label: String, to: Route) -> Element {
    let current_route = use_route::<Route>();
    let dm = use_context::<AdminDarkMode>().0;

    let is_active = {
        if to.to_string() == "/admin/dashboard" {
            current_route == to
        } else {
            current_route.to_string().starts_with(&to.to_string())
        }
    };

    tracing::info!("route: {}, to: {}", current_route, to);

    let dark_mode = dm();

    rsx! {
        Link {
            to: to,
            onclick: move |_| (),
            class: format!(
                "w-full flex items-center space-x-3 px-3 py-2 text-sm font-medium rounded-none transition-colors {}",
                if is_active {
                    if dark_mode { "bg-zinc-600 text-white" } else { "bg-black text-white" }
                } else if dark_mode {
                    "text-zinc-300 hover:bg-zinc-700"
                } else {
                    "text-zinc-700 hover:bg-zinc-100"
                }
            ),
            div {
                class: if is_active || dark_mode { "invert" } else { "" },
                {icon}
            }
            span { "{label}" }
        }
    }
}
