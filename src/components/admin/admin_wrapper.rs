// Add the router prelude to your imports
use dioxus::prelude::*;
use dioxus_i18n::t;

use crate::backend::server_functions::*;
use crate::Route;

#[component]
pub fn AdminWrapper() -> Element {
    let mut is_sidebar_open = use_signal(|| false);
    let mut is_user_menu_open = use_signal(|| false);

    let current_user = use_resource(|| get_current_user());

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

    rsx! {
        document::Title { { format!("{} - Admin Panel", t!("brand") ) } }

        div {
            class: "min-h-screen bg-gray-50 font-mono",

            if is_sidebar_open() {
                div {
                    class: "fixed inset-0 z-40 bg-black bg-opacity-50 lg:hidden",
                    onclick: close_sidebar,
                }
            }

            // Sidebar
            div {
                class: format!(
                    "fixed inset-y-0 left-0 z-50 w-64 bg-white border-r border-gray-200 transform transition-transform duration-200 ease-in-out lg:translate-x-0 {}",
                    if is_sidebar_open() { "translate-x-0" } else { "-translate-x-full" }
                ),

                div {
                    class: "flex items-center justify-between h-16 px-4 border-b border-gray-200",
                    Link {
                        to: Route::Home {},
                        class: "flex items-center space-x-2.5 hover:opacity-75 transition-opacity",
                        div {
                            class: "w-8 h-8 bg-black rounded-none flex items-center justify-center",
                            span { class: "text-white font-bold text-sm", "A" }
                        }
                        span { class: "text-lg font-bold text-gray-900", "Admin" }
                    }
                    button {
                        class: "lg:hidden p-1 text-gray-500 hover:text-gray-700",
                        onclick: move |_| is_sidebar_open.set(false),
                        svg { class: "w-5 h-5", fill: "none", stroke: "currentColor", view_box: "0 0 24 24", path { stroke_linecap: "square", stroke_linejoin: "miter", stroke_width: "2", d: "M6 18L18 6M6 6l12 12" } }
                    }
                }

                // Navigation
                nav {
                    class: "mt-4 px-2",
                    SidebarItem {
                        to: Route::AdminDashboard {},
                        icon: rsx! {
                            img { class: "w-5 h-5", src: asset!("/assets/icons/home.svg") },
                        },
                        label: "Overview",
                    }
                    SidebarItem {
                        to: Route::AdminOrders {},
                        icon: rsx! {
                            img { class: "w-5 h-5", src: asset!("/assets/icons/file-tray-stacked.svg") },
                        },
                        label: "Orders"
                    }
                    SidebarItem {
                        to: Route::AdminUsers {},
                        icon: rsx! {
                            img { class: "w-5 h-5", src: asset!("/assets/icons/people.svg") },
                        },
                        label: "Users"
                    }
                    SidebarItem {
                        to: Route::AdminGroups {},
                        icon: rsx! {
                            img { class: "w-5 h-5", src: asset!("/assets/icons/people-circle-outline.svg") },
                        },
                        label: "Groups"
                    }
                    SidebarItem {
                        to: Route::AdminProducts {},
                        icon: rsx! {
                            img { class: "w-5 h-5", src: asset!("/assets/icons/inventory.svg") },
                        },
                        label: "Products"
                    }
                    SidebarItem {
                        to: Route::AdminInventory {},
                        icon: rsx! {
                            img { class: "w-5 h-5", src: asset!("/assets/icons/receipt.svg") },
                        },
                        label: "Inventory"
                    }
                    SidebarItem {
                        to: Route::AdminDiscounts {},
                        icon: rsx! {
                            img { class: "w-5 h-5", src: asset!("/assets/icons/pricetags.svg") },
                        },
                        label: "Discounts"
                    }
                    SidebarItem {
                        to: Route::AdminContent {},
                        icon: rsx! {
                            img { class: "w-5 h-5", src: asset!("/assets/icons/images.svg") },
                        },
                        label: "Content"
                    }
                    SidebarItem {
                        to: Route::AdminAnalytics {},
                        icon: rsx! {
                            img { class: "w-5 h-5", src: asset!("/assets/icons/bar-chart.svg") },
                        },
                        label: "Analytics"
                    }

                    hr { class: "my-4 border-gray-200" }

                    SidebarItem {
                        to: Route::AdminSettings {},
                        icon: rsx! {
                            img { class: "w-5 h-5", src: asset!("/assets/icons/settings.svg") },
                        },
                        label: "Settings"
                    }
                }
            }

            // Main content
            div {
                class: "lg:pl-64",

                header {
                    class: "h-16 bg-white border-b border-gray-200 flex items-center justify-between px-4",

                    button {
                        class: "lg:hidden p-2 text-gray-500 hover:text-gray-700 hover:bg-gray-100 rounded-none transition-colors",
                        onclick: move |_| is_sidebar_open.set(true),
                        svg { class: "w-5 h-5", fill: "none", stroke: "currentColor", view_box: "0 0 24 24", path { stroke_linecap: "square", stroke_linejoin: "miter", stroke_width: "2", d: "M4 6h16M4 12h16M4 18h16" } }
                    }

                    div { class: "flex-1" }

                    div {
                        class: "flex items-center space-x-2",

                        match current_user.read().as_ref() {
                            Some(Ok(Some(user))) => rsx! {
                                div {
                                    class: "relative",
                                    button {
                                        class: "flex items-center space-x-2 p-1 rounded-none hover:bg-gray-100 transition-colors",
                                        onclick: move |_| is_user_menu_open.set(!is_user_menu_open()),
                                        img { class: "w-8 h-8", src: asset!("/assets/icons/person-circle.svg") },
                                        span { class: "text-sm font-medium text-gray-900 hidden sm:inline", "{user.name}" }
                                        svg { class: "w-4 h-4 text-gray-500", fill: "none", stroke: "currentColor", view_box: "0 0 24 24", path { stroke_linecap: "square", stroke_linejoin: "miter", stroke_width: "2", d: "M19 9l-7 7-7-7" } }
                                    }

                                    if is_user_menu_open() {
                                        div {
                                            class: "absolute right-0 mt-1 w-48 bg-white border border-gray-200 rounded-none shadow-lg z-50",
                                            div {
                                                class: "p-3 border-b border-gray-200",
                                                p { class: "text-sm font-medium text-gray-900", "{user.name}" }
                                                p { class: "text-xs text-gray-600", "{user.email}" }
                                            }
                                            div {
                                                class: "py-1",
                                                hr { class: "my-1 border-gray-200" }
                                                button {
                                                    onclick: handle_logout,
                                                    class: "w-full text-left px-3 py-2 text-sm text-red-700 hover:bg-red-50 transition-colors",
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
                    class: "min-h-screen p-3 md:p-4 bg-gray-100",
                    Outlet::<Route> {}
                }
            }
        }
    }
}


// --- SidebarItem component updated to use Link and be route-aware ---
#[component]
pub fn SidebarItem(icon: Element, label: String, to: Route) -> Element {
    let current_route = use_route::<Route>();

    let is_active = {
        if to.to_string() == "/admin/dashboard" {
            current_route == to
        } else {
            current_route.to_string().starts_with(&to.to_string())
        }
    };

    tracing::info!("route: {}, to: {}", current_route, to);

    rsx! {
        Link {
            to: to,
            onclick: move |_| (),
            class: format!(
                "w-full flex items-center space-x-3 px-3 py-2 text-sm font-medium rounded-none transition-colors {}",
                if is_active {
                    "bg-black text-white"
                } else {
                    "text-gray-700 hover:bg-gray-100"
                }
            ),
            div {
                class: if is_active { "invert" } else { "" },
                {icon}
            }
            span { "{label}" }
        }
    }
}

