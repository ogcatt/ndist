#![allow(non_snake_case)]

use crate::Route;
use crate::backend::server_functions::{
    CreateBlogPostRequest, CreateBlogPostResponse, admin_create_blog_post,
};
use crate::components::*;
use dioxus::prelude::*;

#[component]
pub fn AdminCreateBlogPost() -> Element {
    // Blog post data
    let mut title = use_signal(|| String::new());
    let mut subtitle = use_signal(|| String::new());
    let mut thumbnail_url = use_signal(|| String::new());
    let mut blog_md = use_signal(|| String::new());

    // UI states
    let mut creating = use_signal(|| false);

    // Create notification signals
    let mut notification_message = use_signal(|| String::new());
    let mut notification_type = use_signal(|| String::new()); // "success" or "error"
    let mut show_notification = use_signal(|| false);

    let handle_create_blog_post = move |_| {
        spawn(async move {
            creating.set(true);

            // Validate required fields
            if title().trim().is_empty() {
                notification_message.set("Blog title is required".to_string());
                notification_type.set("error".to_string());
                show_notification.set(true);
                creating.set(false);
                return;
            }

            if blog_md().trim().is_empty() {
                notification_message.set("Blog content is required".to_string());
                notification_type.set("error".to_string());
                show_notification.set(true);
                creating.set(false);
                return;
            }

            // Prepare request data
            let request = CreateBlogPostRequest {
                title: title(),
                subtitle: if subtitle().trim().is_empty() {
                    None
                } else {
                    Some(subtitle())
                },
                thumbnail_url: if thumbnail_url().trim().is_empty() {
                    None
                } else {
                    Some(thumbnail_url())
                },
                blog_md: blog_md(),
            };

            // Call server function
            match admin_create_blog_post(request).await {
                Ok(response) => {
                    if response.success {
                        notification_message.set("Blog post created successfully!".to_string());
                        notification_type.set("success".to_string());
                        show_notification.set(true);

                        // Reset form
                        title.set(String::new());
                        subtitle.set(String::new());
                        thumbnail_url.set(String::new());
                        blog_md.set(String::new());
                    } else {
                        notification_message.set(response.message);
                        notification_type.set("error".to_string());
                        show_notification.set(true);
                    }
                }
                Err(e) => {
                    notification_message.set(format!("Error creating blog post: {}", e));
                    notification_type.set("error".to_string());
                    show_notification.set(true);
                }
            }

            creating.set(false);
        });
    };

    rsx! {
        // Notification
        if show_notification() {
            div {
                class: format!("fixed top-4 right-4 z-50 p-4 rounded-md shadow-lg text-white {}",
                    if notification_type() == "success" { "bg-green-500" } else { "bg-red-500" }
                ),
                div {
                    class: "flex justify-between items-center",
                    span { "{notification_message()}" }
                    button {
                        class: "ml-4 text-white hover:text-gray-200",
                        onclick: move |_| show_notification.set(false),
                        "×"
                    }
                }
            }
        }

        div {
            class: "border rounded-md border-gray-200 w-full mb-4 flex justify-between items-center p-2 pl-4",
            div {
                class: "text-lg font-medium",
                "Create New Blog Post"
            }
            button {
                class: format!("text-sm px-3 py-2 text-white rounded transition-colors {}",
                    if creating() { "bg-gray-500 cursor-not-allowed" } else { "bg-gray-900 hover:bg-gray-800" }
                ),
                disabled: creating(),
                onclick: handle_create_blog_post,
                if creating() {
                    "Creating..."
                } else {
                    "Create"
                }
            }
        }

        div {
            class: "flex flex-col md:flex-row w-full gap-2",
            div {
                class: "flex w-full flex-col gap-2",

                // Basic Info Section
                div {
                    class: "flex-grow bg-white border rounded-md border-gray-200 p-4 min-h-36",
                    h2 {
                        class: "text-lg font-medium mb-4",
                        "Basic Information"
                    }
                    div {
                        class: "flex flex-col gap-4 w-full",
                        CTextBox {
                            label: "Blog Title",
                            value: "{title}",
                            placeholder: "Enter blog post title...",
                            optional: false,
                            oninput: move |event: FormEvent| {
                                title.set(event.value());
                            }
                        }

                        CTextBox {
                            label: "Subtitle",
                            value: "{subtitle}",
                            placeholder: "Optional subtitle or summary...",
                            optional: true,
                            oninput: move |event: FormEvent| {
                                subtitle.set(event.value());
                            }
                        }

                        CTextBox {
                            label: "Thumbnail URL",
                            value: "{thumbnail_url}",
                            placeholder: "https://example.com/image.jpg",
                            optional: true,
                            oninput: move |event: FormEvent| {
                                thumbnail_url.set(event.value());
                            }
                        }
                    }
                },

                // Content Section
                div {
                    class: "flex-grow bg-white border rounded-md border-gray-200 p-4 min-h-96",
                    h2 {
                        class: "text-lg font-medium mb-4",
                        "Blog Content"
                    }
                    CTextArea {
                        label: "Blog Content (Markdown)",
                        placeholder: "Write your blog post content in markdown format...",
                        value: "{blog_md}",
                        oninput: move |event: FormEvent| blog_md.set(event.value())
                    }
                }
            }

            // Right sidebar
            div {
                class: "md:w-[38%] w-full min-w-0",

                // Preview/Info
                div {
                    class: "bg-white border flex flex-col gap-4 rounded-md border-gray-200 p-4 min-h-36 mb-2",
                    h2 {
                        class: "text-lg font-medium mb-4",
                        "Blog Post Preview"
                    }

                    if !title().trim().is_empty() {
                        div {
                            class: "border-l-4 border-blue-500 pl-4 mb-3",
                            h3 {
                                class: "font-semibold text-gray-900 text-lg",
                                "{title()}"
                            }
                            if !subtitle().trim().is_empty() {
                                p {
                                    class: "text-gray-600 text-sm mt-1",
                                    "{subtitle()}"
                                }
                            }
                        }
                    }

                    if !thumbnail_url().trim().is_empty() {
                        div {
                            class: "mb-3",
                            p {
                                class: "text-sm text-gray-600 mb-2",
                                "Thumbnail:"
                            }
                            div {
                                class: "w-full h-32 bg-gray-100 rounded border flex items-center justify-center",
                                img {
                                    src: "{thumbnail_url()}",
                                    alt: "Thumbnail preview",
                                    class: "max-w-full max-h-full object-cover rounded",
                                    //onerror: "this.style.display='none'; this.nextElementSibling.style.display='block';",
                                }
                                div {
                                    style: "display: none;",
                                    class: "text-gray-500 text-sm text-center p-4",
                                    "Invalid image URL"
                                }
                            }
                        }
                    }

                    div {
                        class: "text-sm text-gray-600",
                        p { "Content length: {blog_md().len()} characters" }
                        if !blog_md().trim().is_empty() {
                            p { "Words: ~{blog_md().split_whitespace().count()}" }
                        }
                    }
                }

                // Writing Tips
                div {
                    class: "bg-white border rounded-md border-gray-200 p-4 min-h-36 mb-2",
                    h2 {
                        class: "text-lg font-medium mb-4",
                        "Writing Tips"
                    }
                    div {
                        class: "text-sm text-gray-600 space-y-2",
                        p { "• Use # for main headings" }
                        p { "• Use ## for sub-headings" }
                        p { "• Use **text** for bold" }
                        p { "• Use *text* for italic" }
                        p { "• Use [text](url) for links" }
                        p { "• Use ![alt](url) for images" }
                        p { "• Use > for quotes" }
                        p { "• Use ``` for code blocks" }
                    }
                }
            }
        }
    }
}
