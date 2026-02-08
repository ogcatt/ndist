use dioxus::prelude::*;

#[component]
pub fn UserDashboard() -> Element {
    rsx! {
        div { class: "max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12",
            div { class: "md:flex md:items-center md:justify-between mb-8",
                div { class: "flex-1 min-w-0",
                    h2 { class: "text-2xl font-bold leading-7 text-gray-900 sm:text-3xl sm:truncate",
                        "Dashboard" }
                }
            }
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
                                "user@example.com" }
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
    }
}