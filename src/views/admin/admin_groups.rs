#![allow(non_snake_case)]

use chrono::NaiveDateTime;
use dioxus::prelude::*;
use std::time::Duration;

use crate::Route;
use crate::backend::cache::use_cached_server;
use crate::backend::server_functions;

// Helper function to format date for display
fn format_date(date: &NaiveDateTime) -> String {
    date.format("%b %d, %Y").to_string()
}

#[component]
pub fn AdminGroups() -> Element {
    let groups_req = use_cached_server(
        "admin_groups_list",
        || server_functions::admin_get_groups(),
        Duration::from_secs(15),
    );

    rsx! {
        div {
            div {
                class: "border rounded-md border-gray-200 w-full mb-4 flex justify-between items-center p-2 pl-4",
                div {
                    class: "text-lg font-medium",
                    "Groups"
                }
                Link {
                    to: Route::AdminCreateGroup {},
                    button {
                        class: "text-sm bg-gray-900 px-3 py-2 text-white rounded hover:bg-gray-800 transition-colors",
                        "Create Group"
                    }
                }
            }

            div {
                class: "w-full",
                {match &*groups_req.read() {
                    Some(Ok(groups)) => {
                        // Sort groups by name
                        let mut sorted_groups = groups.clone();
                        sorted_groups.sort_by(|a, b| a.name.cmp(&b.name));

                        rsx! {
                            if sorted_groups.is_empty() {
                                div {
                                    class: "mt-12 text-center",
                                    "No groups created yet"
                                }
                            } else {
                                for group in sorted_groups.iter() {
                                    {
                                        let formatted_created = format_date(&group.created_at);
                                        let formatted_updated = format_date(&group.updated_at);

                                        rsx! {
                                            div {
                                                class: "bg-white w-full min-h-12 border rounded-md border-gray-200 p-4 mb-4",
                                                div {
                                                    class: "flex items-center gap-4",
                                                    // Status indicator
                                                    div {
                                                        class: "w-1 h-16 rounded bg-blue-500"
                                                    }

                                                    div {
                                                        class: "flex-1",
                                                        div {
                                                            class: "flex items-start justify-between",
                                                            div {
                                                                class: "flex-1 min-w-0",
                                                                // Group Name
                                                                h3 {
                                                                    class: "text-lg font-medium mb-1",
                                                                    "{group.name}"
                                                                }

                                                                // Description if present
                                                                if let Some(description) = &group.description {
                                                                    if !description.is_empty() {
                                                                        div {
                                                                            class: "text-sm text-gray-600 mb-2",
                                                                            "{description}"
                                                                        }
                                                                    }
                                                                }

                                                                div {
                                                                    class: "flex items-center gap-6 text-sm",
                                                                    // Member count
                                                                    div {
                                                                        class: "text-gray-700 font-medium",
                                                                        "{group.member_count} "
                                                                        span {
                                                                            class: "text-gray-500 font-normal",
                                                                            if group.member_count == 1 { "member" } else { "members" }
                                                                        }
                                                                    }

                                                                    // Created Date
                                                                    div {
                                                                        class: "text-gray-500",
                                                                        "Created: {formatted_created}"
                                                                    }

                                                                    // Updated Date
                                                                    div {
                                                                        class: "text-gray-500",
                                                                        "Updated: {formatted_updated}"
                                                                    }
                                                                }
                                                            }

                                                            // Edit link on the right side
                                                            Link {
                                                                to: Route::AdminEditGroup { id: group.id.clone() },
                                                                title: "Edit group",
                                                                class: "flex items-center justify-center w-8 h-8 rounded hover:bg-gray-100 transition-colors ml-4",
                                                                img {
                                                                    class: "w-5 h-5",
                                                                    src: asset!("/assets/icons/create-outline.svg")
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
                        }
                    },
                    Some(Err(_)) => rsx! {
                        p { "Error loading groups" }
                    },
                    None => rsx! {
                        p { "Loading groups..." }
                    }
                }}
            }
        }
    }
}
