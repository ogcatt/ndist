use dioxus::prelude::*;
use crate::components::account_popup::{use_global_session_state, SessionState};
use crate::Route;
use crate::backend::cache::{use_hybrid_cache, use_stale_while_revalidate};
use crate::backend::server_functions;
use std::time::Duration;

#[derive(Clone, Copy, PartialEq)]
enum DashboardPage {
    Overview,
    Orders,
    Settings,
}

#[component]
pub fn UserDashboard() -> Element {
    // Use cached session state instead of making a server request
    let session = use_global_session_state();

    let mut current_page = use_signal(|| DashboardPage::Overview);

    // Get groups data using hybrid cache
    let groups_data = use_stale_while_revalidate(
        "get_user_groups",
        || async { server_functions::get_user_groups().await },
        Duration::from_secs(180),
    );

    // Get all products for counting group access
    let all_products_data = use_hybrid_cache(
        "get_products",
        || async { server_functions::get_products().await },
        Duration::from_secs(180),
    );

    // Redirect to home if not authenticated
    if !session.authenticated {
        use_effect(move || {
            let _ = web_sys::window()
                .unwrap()
                .location()
                .set_href("/");
        });
    }

    rsx! {
        div { class: "min-h-screen bg-gray-50",
            // Desktop layout with sidebar
            div { class: "hidden md:flex",
                // Sidebar
                div { class: "w-64 bg-white border-r border-gray-200 min-h-screen flex flex-col",
                    // Navigation items
                    div { class: "flex-1 pt-8",
                        nav { class: "space-y-1",
                            // Overview
                            button {
                                class: if *current_page.read() == DashboardPage::Overview {
                                    "w-full text-left px-6 py-3 text-sm font-medium text-blue-600 bg-blue-50 border-r-2 border-blue-600"
                                } else {
                                    "w-full text-left px-6 py-3 text-sm font-medium text-gray-700 hover:bg-gray-50 hover:text-gray-900"
                                },
                                onclick: move |_| current_page.set(DashboardPage::Overview),
                                "Overview"
                            }

                            // Orders
                            button {
                                class: if *current_page.read() == DashboardPage::Orders {
                                    "w-full text-left px-6 py-3 text-sm font-medium text-blue-600 bg-blue-50 border-r-2 border-blue-600"
                                } else {
                                    "w-full text-left px-6 py-3 text-sm font-medium text-gray-700 hover:bg-gray-50 hover:text-gray-900"
                                },
                                onclick: move |_| current_page.set(DashboardPage::Orders),
                                "Orders"
                            }

                            // Settings
                            button {
                                class: if *current_page.read() == DashboardPage::Settings {
                                    "w-full text-left px-6 py-3 text-sm font-medium text-blue-600 bg-blue-50 border-r-2 border-blue-600"
                                } else {
                                    "w-full text-left px-6 py-3 text-sm font-medium text-gray-700 hover:bg-gray-50 hover:text-gray-900"
                                },
                                onclick: move |_| current_page.set(DashboardPage::Settings),
                                "Settings"
                            }
                        }
                    }

                    // Admin Panel link (sticky to bottom)
                    if session.admin {
                        div { class: "border-t border-gray-200 p-4",
                            Link {
                                to: Route::AdminDashboard {},
                                class: "flex items-center px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 transition-colors",
                                "Admin Panel"
                            }
                        }
                    }
                }

                // Main content area
                div { class: "flex-1 py-8 px-6 overflow-y-auto",
                    match *current_page.read() {
                        DashboardPage::Overview => rsx! {
                            OverviewPage { session: session.clone(), groups_data: groups_data.clone(), all_products_data: all_products_data.clone() }
                        },
                        DashboardPage::Orders => rsx! {
                            OrdersPage {}
                        },
                        DashboardPage::Settings => rsx! {
                            SettingsPage {}
                        },
                    }
                }
            }

            // Mobile layout (stacked sections)
            div { class: "md:hidden",
                div { class: "pt-8 px-6",
                    // Admin link for mobile
                    if session.admin {
                        div { class: "mb-4",
                            Link {
                                to: Route::AdminDashboard {},
                                class: "text-blue-600 hover:text-blue-800 text-sm font-medium",
                                "Visit Admin Dashboard →"
                            }
                        }
                    }

                    // Overview section
                    OverviewPage { session: session.clone(), groups_data: groups_data.clone(), all_products_data: all_products_data.clone() }

                    // Orders section
                    div { class: "mt-12",
                        OrdersPage {}
                    }

                    // Settings section
                    div { class: "mt-12 pb-8",
                        SettingsPage {}
                    }
                }
            }
        }
    }
}

#[component]
fn OverviewPage(session: SessionState, groups_data: Signal<Option<Vec<crate::backend::models::Group>>>, all_products_data: Signal<Option<Vec<crate::backend::models::Product>>>) -> Element {
    rsx! {
        div { class: "max-w-4xl",
            h2 { class: "text-2xl leading-7 text-gray-900 sm:text-3xl mb-8",
                "Dashboard"
            }

            if session.authenticated {
                // Account Overview
                div { class: "bg-white border border-gray-200 overflow-hidden sm:rounded-lg mb-8",
                    div { class: "px-4 py-5 sm:px-6",
                        h3 { class: "text-lg leading-6 font-medium text-gray-900",
                            "Account Overview"
                        }
                        p { class: "mt-1 max-w-2xl text-sm text-gray-500",
                            "Your account information and settings"
                        }
                    }
                    div { class: "border-t border-gray-200 px-4 py-5 sm:p-0",
                        dl { class: "sm:divide-y sm:divide-gray-200",
                            div { class: "py-4 sm:py-5 sm:grid sm:grid-cols-3 sm:gap-4 sm:px-6",
                                dt { class: "text-sm font-medium text-gray-500",
                                    "Email"
                                }
                                dd { class: "mt-1 text-sm text-gray-900 sm:mt-0 sm:col-span-2",
                                    "{session.email}"
                                }
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
                            }
                            div { class: "py-4 sm:py-5 sm:grid sm:grid-cols-3 sm:gap-4 sm:px-6",
                                dt { class: "text-sm font-medium text-gray-500",
                                    "Account Status"
                                }
                                dd { class: "mt-1 text-sm text-gray-900 sm:mt-0 sm:col-span-2",
                                    "Active"
                                }
                            }
                            if session.admin {
                                div { class: "py-4 sm:py-5 sm:grid sm:grid-cols-3 sm:gap-4 sm:px-6",
                                    dt { class: "text-sm font-medium text-gray-500",
                                        "Admin"
                                    }
                                    dd { class: "mt-1 text-sm text-gray-900 sm:mt-0 sm:col-span-2",
                                        "true"
                                    }
                                }
                            }
                        }
                    }
                }

                // Groups section
                if !session.group_ids.is_empty() {
                    div { class: "bg-white border border-gray-200 overflow-hidden sm:rounded-lg",
                        div { class: "px-4 py-5 sm:px-6",
                            h3 { class: "text-lg leading-6 font-medium text-gray-900",
                                "Your Groups"
                            }
                            p { class: "mt-1 max-w-2xl text-sm text-gray-500",
                                "Groups you're a member of"
                            }
                        }
                        div { class: "border-t border-gray-200 px-4 py-5 sm:px-6",
                            {
                                let groups_opt = groups_data.read().clone();
                                let products_opt = all_products_data.read().clone();

                                if let Some(groups) = groups_opt {
                                    let user_groups: Vec<_> = groups.iter()
                                        .filter(|g| session.group_ids.contains(&g.id))
                                        .collect();

                                    rsx! {
                                        div { class: "space-y-4",
                                            for group in user_groups {
                                                {
                                                    // Calculate product count for this group
                                                    let product_count = if let Some(ref products) = products_opt {
                                                        products.iter()
                                                            .filter(|p| {
                                                                p.access_groups.as_ref()
                                                                    .map(|groups| groups.contains(&group.id))
                                                                    .unwrap_or(false)
                                                            })
                                                            .count()
                                                    } else {
                                                        0
                                                    };

                                                    rsx! {
                                                        Link {
                                                            to: Route::GroupPage { id: group.id.clone() },
                                                            class: "block",
                                                            div { class: "border border-gray-200 rounded-lg p-4 hover:border-gray-300 transition-colors",
                                                                div { class: "flex items-start justify-between",
                                                                    div { class: "flex-1",
                                                                        h4 { class: "text-base font-medium text-gray-900",
                                                                            "{group.name}"
                                                                        }
                                                                        if let Some(description) = &group.description {
                                                                            p { class: "mt-1 text-sm text-gray-600",
                                                                                "{description}"
                                                                            }
                                                                        }
                                                                        p { class: "mt-2 text-xs text-gray-500",
                                                                            "Provides access to {product_count} products."
                                                                        }
                                                                    }
                                                                    div { class: "ml-4",
                                                                        img {
                                                                            src: asset!("/assets/icons/chevron-forward-outline.svg"),
                                                                            alt: "",
                                                                            class: "h-5 w-5 text-gray-400"
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    rsx! {
                                        p { class: "text-sm text-gray-500",
                                            "Loading groups..."
                                        }
                                    }
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

#[component]
fn OrdersPage() -> Element {
    rsx! {
        div { class: "max-w-4xl",
            h2 { class: "text-2xl leading-7 text-gray-900 sm:text-3xl mb-8",
                "Orders"
            }
            div { class: "bg-white border border-gray-200 overflow-hidden sm:rounded-lg",
                div { class: "px-4 py-5 sm:px-6",
                    h3 { class: "text-lg leading-6 font-medium text-gray-900",
                        "Order History"
                    }
                    p { class: "mt-1 max-w-2xl text-sm text-gray-500",
                        "View your past orders and their status"
                    }
                }
                div { class: "border-t border-gray-200 px-4 py-5 sm:px-6",
                    p { class: "text-sm text-gray-500",
                        "No orders yet."
                    }
                }
            }
        }
    }
}

#[component]
fn SettingsPage() -> Element {
    rsx! {
        div { class: "max-w-4xl",
            h2 { class: "text-2xl leading-7 text-gray-900 sm:text-3xl mb-8",
                "Settings"
            }
            div { class: "bg-white border border-gray-200 overflow-hidden sm:rounded-lg",
                div { class: "px-4 py-5 sm:px-6",
                    h3 { class: "text-lg leading-6 font-medium text-gray-900",
                        "Account Settings"
                    }
                    p { class: "mt-1 max-w-2xl text-sm text-gray-500",
                        "Manage your account preferences"
                    }
                }
            }
        }
    }
}
