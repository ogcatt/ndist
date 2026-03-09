#![allow(non_snake_case)] // Allow non-snake_case identifiers

use chrono::{NaiveDateTime, Utc};
use dioxus::prelude::*;
use std::time::Duration;

use crate::Route;
use crate::backend::cache::use_cached_server;
use crate::backend::front_entities::*;
use crate::backend::server_functions;

// Helper function to format date for display
fn format_date(date: &NaiveDateTime) -> String {
    date.format("%b %d, %Y").to_string()
}

// Helper function to truncate content for preview
fn truncate_content(content: &str, max_length: usize) -> String {
    if content.len() <= max_length {
        content.to_string()
    } else {
        format!("{}...", &content[..max_length])
    }
}

#[component]
pub fn AdminContent() -> Element {
    let blog_posts_req = use_cached_server(
        "admin_blog_posts_list", // Unique key for this server function
        || server_functions::admin_get_blog_posts(),
        Duration::from_secs(15), // Cache for 15 seconds
    );

    use_effect(move || {
        println!("{:#?}", blog_posts_req);
    });

    rsx! {
        div {
            div {
                class: "border rounded-md border-gray-200 w-full mb-4 flex justify-between items-center p-2 pl-4",
                div {
                    class: "text-lg font-medium",
                    "Blog Posts"
                }
                Link {
                    to: Route::AdminCreateBlogPost {},
                    button {
                        class: "text-sm bg-zinc-600 px-3 py-2 text-white rounded hover:bg-zinc-500 transition-colors",
                        "Create Blog Post"
                    }
                }
            }

            div {
                class: "w-full",
                {match &*blog_posts_req.read() {
                    Some(Ok(blog_posts)) => {
                        // Sort blog posts by posted_at in descending order (most recent first)
                        let mut sorted_blog_posts = blog_posts.clone();
                        sorted_blog_posts.sort_by(|a, b| b.posted_at.cmp(&a.posted_at));

                        rsx! {
                            if sorted_blog_posts.len() == 0 {
                                div {
                                    class: "mt-12 text-center",
                                    "No blog posts created yet"
                                }
                            } else {
                                for blog_post in sorted_blog_posts.iter() {
                                    {
                                        let formatted_date = format_date(&blog_post.posted_at);
                                        let content_preview = truncate_content(&blog_post.blog_md, 100);

                                        rsx! {
                                            div {
                                                class: "bg-white w-full min-h-12 border rounded-md border-gray-200 p-4 mb-4",
                                                div {
                                                    class: "flex items-center gap-4",
                                                    // Status indicator (blue for all posts since no draft/published field)
                                                    div {
                                                        class: "w-1 h-16 rounded bg-blue-500"
                                                    }

                                                    div {
                                                        class: "flex-1",
                                                        div {
                                                            class: "flex items-start justify-between",
                                                            div {
                                                                class: "flex-1 min-w-0",
                                                                // Blog Post Title
                                                                h3 {
                                                                    class: "text-lg font-medium mb-1",
                                                                    "{blog_post.title}"
                                                                }

                                                                // Subtitle if present
                                                                if let Some(subtitle) = &blog_post.subtitle {
                                                                    div {
                                                                        class: "text-sm text-gray-600 mb-2",
                                                                        "{subtitle}"
                                                                    }
                                                                }

                                                                div {
                                                                    class: "flex items-center gap-6 text-sm",
                                                                    // Posted Date
                                                                    div {
                                                                        class: "text-gray-500",
                                                                        "Posted: {formatted_date}"
                                                                    }

                                                                    // Updated Date
                                                                    div {
                                                                        class: "text-gray-500",
                                                                        "Updated: {format_date(&blog_post.updated_at)}"
                                                                    }

                                                                    // Thumbnail indicator
                                                                    if blog_post.thumbnail_url.is_some() {
                                                                        span {
                                                                            class: "px-2 py-1 bg-blue-100 text-blue-800 rounded text-xs",
                                                                            "Has thumbnail"
                                                                        }
                                                                    }
                                                                }

                                                                // Content preview
                                                                div {
                                                                    class: "mt-2 text-xs text-gray-400 font-mono",
                                                                    "{content_preview}"
                                                                }
                                                            }

                                                            // Edit link on the right side
                                                            Link {
                                                                to: Route::AdminEditBlogPost { id: blog_post.id.clone() },
                                                                title: "Edit blog post",
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
                        p { "Error loading blog posts" }
                    },
                    None => rsx! {
                        p { "Loading blog posts..." }
                    }
                }}
            }
        }
    }
}
