// Add the router prelude to your imports
use dioxus::prelude::*;
use dioxus_i18n::t;

use crate::backend::server_functions::*;
use crate::Route;

#[component]
pub fn AdminWrapper() -> Element {
    let mut is_sidebar_open = use_signal(|| false);
    let mut is_dark_mode = use_signal(|| false);
    let mut is_user_menu_open = use_signal(|| false);

    let current_user = use_resource(|| get_current_user());

    let navigator = use_navigator();

    let handle_logout = move |_| {
        spawn(async move {
            match logout_user().await {
                Ok(_) => {
                    navigator.push("/admin/signin");
                }
                Err(e) => {
                    // Handle error
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

        // The dark mode toggle logic was correct and has been preserved.
        div {
            class: if is_dark_mode() { "dark" } else { "" },
            div {
                class: "min-h-screen bg-gray-50 dark:bg-gray-900 font-mono transition-colors duration-200",

                if is_sidebar_open() {
                    div {
                        class: "fixed inset-0 z-40 bg-black bg-opacity-50 lg:hidden",
                        onclick: close_sidebar,
                    }
                }

                div {
                    class: format!(
                        "fixed inset-y-0 left-0 z-50 w-64 bg-white dark:bg-gray-800 border-r border-gray-200 dark:border-gray-700 transform transition-transform duration-200 ease-in-out lg:translate-x-0 {}",
                        if is_sidebar_open() { "translate-x-0" } else { "-translate-x-full" }
                    ),

                    div {
                        class: "flex items-center justify-between h-16 px-4 border-b border-gray-200 dark:border-gray-700",
                        div {
                            class: "flex items-center space-x-2.5",
                            div {
                                class: "w-8 h-8 bg-black dark:bg-white rounded-none flex items-center justify-center",
                                span { class: "text-white dark:text-black font-bold text-sm", "A" }
                            }
                            span { class: "text-lg font-bold text-gray-900 dark:text-white", "Admin" }
                        }
                        button {
                            class: "lg:hidden p-1 text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200",
                            onclick: move |_| is_sidebar_open.set(false),
                            svg { class: "w-5 h-5", fill: "none", stroke: "currentColor", view_box: "0 0 24 24", path { stroke_linecap: "square", stroke_linejoin: "miter", stroke_width: "2", d: "M6 18L18 6M6 6l12 12" } }
                        }
                    }

                    // Navigation: Each SidebarItem is now a real link pointing to a Route
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
                            to: Route::AdminCustomers {},
                            icon: rsx! {
                                img { class: "w-5 h-5", src: asset!("/assets/icons/people.svg") },
                            },
                            label: "Customers"
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

                        hr { class: "my-4 border-gray-200 dark:border-gray-600" }

                        SidebarItem {
                            to: Route::AdminSettings {},
                            icon: rsx! {
                                img { class: "w-5 h-5", src: asset!("/assets/icons/settings.svg") },
                            },

                            label: "Settings"
                        }
                    }
                }

                div {
                    class: "lg:pl-64",

                    header {
                        class: "h-16 bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between px-4",

                        button {
                            class: "lg:hidden p-2 text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-none transition-colors",
                            onclick: move |_| is_sidebar_open.set(true),
                            svg { class: "w-5 h-5", fill: "none", stroke: "currentColor", view_box: "0 0 24 24", path { stroke_linecap: "square", stroke_linejoin: "miter", stroke_width: "2", d: "M4 6h16M4 12h16M4 18h16" } }
                        }

                        div { class: "flex-1" }

                        div {
                            class: "flex items-center space-x-2",

                                // This button's functionality was already correct.
                                button {
                                class: "p-2 text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-none transition-colors",
                                onclick: move |_| is_dark_mode.set(!is_dark_mode()),
                                if is_dark_mode() {
                                    svg { class: "w-5 h-5", fill: "currentColor", view_box: "0 0 20 20", path { d: "M10 2L13.09 8.26L20 9L14 14.74L15.18 21.02L10 18L4.82 21.02L6 14.74L0 9L6.91 8.26L10 2Z" } }
                                } else {
                                    svg { class: "w-5 h-5", fill: "currentColor", view_box: "0 0 20 20", path { d: "M17.293 13.293A8 8 0 016.707 2.707a8.001 8.001 0 1010.586 10.586z" } }
                                }
                            }

                            match current_user.read().as_ref() {
                                Some(Ok(Some(user))) => rsx! {
                                div {
                                    class: "relative",
                                    button {
                                        class: "flex items-center space-x-2 p-1 rounded-none hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors",
                                        onclick: move |_| is_user_menu_open.set(!is_user_menu_open()),
                                        img { class: "w-8 h-8", src: asset!("/assets/icons/person-circle.svg") },
                                        span { class: "text-sm font-medium text-gray-900 dark:text-white", "{user.name}" }
                                        svg { class: "w-4 h-4 text-gray-500 dark:text-gray-400", fill: "none", stroke: "currentColor", view_box: "0 0 24 24", path { stroke_linecap: "square", stroke_linejoin: "miter", stroke_width: "2", d: "M19 9l-7 7-7-7" } }
                                    }

                                    if is_user_menu_open() {
                                            div {
                                                class: "absolute right-0 mt-1 w-48 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-none shadow-lg z-50",
                                                div {
                                                    class: "p-3 border-b border-gray-200 dark:border-gray-600",
                                                    p { class: "text-sm font-medium text-gray-900 dark:text-white", "{user.name}" }
                                                    p { class: "text-xs text-gray-600 dark:text-gray-300", "{user.email}" }
                                                }
                                                div {
                                                    class: "py-1",
                                                    // The buttons in the dropdown are now functional Links
                                                    Link {
                                                        to: "#",
                                                        class: "w-full text-left block px-3 py-2 text-sm text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors",
                                                        "Profile"
                                                    }
                                                    Link {
                                                        to: "#",
                                                        class: "w-full text-left block px-3 py-2 text-sm text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors",
                                                        "Settings"
                                                    }
                                                    hr { class: "my-1 border-gray-200 dark:border-gray-600" }
                                                    // Sign out button now navigates to the sign in page


                                                    button {
                                                        onclick: handle_logout,
                                                        class: "w-full text-left px-3 py-2 text-sm text-red-700 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors",
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
                        class: "min-h-screen p-4 bg-gray-100 dark:bg-gray-900",
                        Outlet::<Route> {}
                    }
                }
            }
        }
    }
}


// --- SidebarItem component updated to use Link and be route-aware ---
#[component]
pub fn SidebarItem(icon: Element, label: String, to: Route) -> Element {
    // Get the current route to determine if this link is active
    let current_route = use_route::<Route>();

    // Check if the current route matches this item's destination route

    //OLD
    //let is_active = current_route == to;

    //NEW
    let is_active = {
        // As this flags all sub-pages, make an exception
        if to.to_string() == "/admin/dashboard" {
            current_route == to
        } else {
            current_route.to_string().starts_with(&to.to_string())
        }
    };

    tracing::info!("route: {}, to: {}", current_route, to);

    rsx! {
        // Use the Link component for navigation instead of a button
        Link {
            to: to,
            // onclick handler to close the sidebar on mobile after navigation
            onclick: move |_| (),
            class: format!(
                "w-full flex items-center space-x-3 px-3 py-2 text-sm font-medium rounded-none transition-colors {}",
                if is_active {
                    "bg-black text-white dark:bg-white dark:text-black"
                } else {
                    "text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700"
                }
            ),
            div {
                class: if is_active { "invert" } else { "dark:invert" },
                {icon}
            }
            span { "{label}" }
        }
    }
}


// Example usage remains the same
#[component]
pub fn AdminDashboard() -> Element {
    rsx! {
        // This is now the content for the "/" route
        div {
            class: "space-y-6",

            div {
                class: "border-b border-gray-200 dark:border-gray-700 pb-4",
                h1 { class: "text-2xl font-bold text-gray-900 dark:text-white", "Dashboard" }
                p { class: "mt-1 text-sm text-gray-600 dark:text-gray-300", "Welcome to your admin dashboard" }
            }

            div {
                class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4",
                StatsCard { title: "Total Users", value: "1,234", change: "+12%", trend: "up" }
                StatsCard { title: "Revenue", value: "$12,345", change: "+8%", trend: "up" }
                StatsCard { title: "Orders", value: "856", change: "-3%", trend: "down" }
                StatsCard { title: "Conversion", value: "2.4%", change: "+1.2%", trend: "up" }
            }

            div {
                class: "bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-none",
                div {
                    class: "p-6",
                    h2 { class: "text-lg font-bold text-gray-900 dark:text-white mb-4", "Recent Activity"}
                    p { class: "text-gray-600 dark:text-gray-300", "Your content goes here..." }
                }
            }
        }
    }
}

#[component]
pub fn StatsCard(title: String, value: String, change: String, trend: String) -> Element {
    rsx! {
        div {
            class: "bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-none p-4",
            div {
                class: "flex items-center justify-between",
                div {
                    h3 { class: "text-sm font-medium text-gray-600 dark:text-gray-300", "{title}" }
                    p { class: "text-2xl font-bold text-gray-900 dark:text-white mt-1", "{value}" }
                }
                div {
                    class: format!(
                        "text-sm font-medium {}",
                        if trend == "up" { "text-green-600 dark:text-green-400" } else { "text-red-600 dark:text-red-400" }
                    ),
                    "{change}"
                }
            }
        }
    }
}
