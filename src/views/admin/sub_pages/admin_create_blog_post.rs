#![allow(non_snake_case)]

use crate::Route;
use crate::backend::front_entities::BlogPost;
use crate::backend::server_functions::{
    CreateBlogPostRequest, EditBlogPostRequest,
    admin_create_blog_post, admin_edit_blog_post, admin_get_blog_post,
};
use crate::components::*;
use chrono::NaiveDateTime;
use dioxus::prelude::*;
use markdown::to_html;

#[derive(PartialEq, Props, Clone)]
pub struct AdminBlogPostProps {
    pub id: Option<Signal<String>>,
}

#[component]
pub fn AdminBlogPost(props: AdminBlogPostProps) -> Element {
    let is_edit_mode = props.id.is_some();
    let mut post_id = use_signal(|| String::new());
    let props_id = props.id;

    use_effect(move || {
        if let Some(id) = props_id {
            post_id.set(id());
        }
    });

    let mut title = use_signal(|| String::new());
    let mut subtitle = use_signal(|| String::new());
    let mut thumbnail_url = use_signal(|| String::new());
    let mut blog_md = use_signal(|| String::new());

    let mut saving = use_signal(|| false);
    let mut loaded = use_signal(|| false);

    let mut notification_message = use_signal(|| String::new());
    let mut notification_type = use_signal(|| String::new());
    let mut show_notification = use_signal(|| false);

    let blog_post_resource = use_resource(move || async move {
        if is_edit_mode && !post_id().is_empty() {
            admin_get_blog_post(post_id()).await
        } else {
            Err(ServerFnError::new("Not in edit mode"))
        }
    });

    use_effect(move || {
        if !is_edit_mode {
            loaded.set(false);
            return;
        }

        if let Some(res) = blog_post_resource() {
            match res {
                Ok(blog_post) => {
                    if !*loaded.peek() {
                        title.set(blog_post.title);
                        subtitle.set(blog_post.subtitle.unwrap_or_default());
                        thumbnail_url.set(blog_post.thumbnail_url.unwrap_or_default());
                        blog_md.set(blog_post.blog_md);
                        loaded.set(true);
                    }
                }
                Err(e) => {
                    notification_message.set(format!("Error loading blog post: {}", e));
                    notification_type.set("error".to_string());
                    show_notification.set(true);
                }
            }
        }
    });

    let handle_save_blog_post = move |_| {
        spawn(async move {
            saving.set(true);

            if title().trim().is_empty() {
                notification_message.set("Blog title is required".to_string());
                notification_type.set("error".to_string());
                show_notification.set(true);
                saving.set(false);
                return;
            }

            let save_result: Result<(), ServerFnError> = if is_edit_mode {
                admin_edit_blog_post(EditBlogPostRequest {
                    id: post_id(),
                    title: title(),
                    subtitle: if subtitle().trim().is_empty() { None } else { Some(subtitle()) },
                    thumbnail_url: if thumbnail_url().trim().is_empty() { None } else { Some(thumbnail_url()) },
                    blog_md: blog_md(),
                }).await.map(|_| ())
            } else {
                admin_create_blog_post(CreateBlogPostRequest {
                    title: title(),
                    subtitle: if subtitle().trim().is_empty() { None } else { Some(subtitle()) },
                    thumbnail_url: if thumbnail_url().trim().is_empty() { None } else { Some(thumbnail_url()) },
                    blog_md: blog_md(),
                }).await.map(|_| ())
            };

            match save_result {
                Ok(_) => {
                    notification_message.set(
                        if is_edit_mode {
                            "Blog post updated successfully!".to_string()
                        } else {
                            "Blog post created successfully!".to_string()
                        }
                    );
                    notification_type.set("success".to_string());
                    show_notification.set(true);

                    if !is_edit_mode {
                        title.set(String::new());
                        subtitle.set(String::new());
                        thumbnail_url.set(String::new());
                        blog_md.set(String::new());
                    }
                }
                Err(e) => {
                    notification_message.set(format!("Error saving blog post: {}", e));
                    notification_type.set("error".to_string());
                    show_notification.set(true);
                }
            }

            saving.set(false);
        });
    };

    if is_edit_mode && !loaded() {
        if blog_post_resource().is_some_and(|r| r.is_err()) {
            return rsx! {
                div { class: "text-red-500", "Error loading blog post." }
            };
        } else {
            return rsx! { div { "Loading..." } };
        }
    }

    rsx! {
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
                if is_edit_mode {
                    if !title().is_empty() {
                        span {
                            class: "text-gray-500 text-base ml-2",
                            "- {title()}"
                        }
                    }
                    "Edit Blog Post"
                } else {
                    "Create New Blog Post"
                }
            }
            div {
                class: "flex gap-2",
                if is_edit_mode {
                    Link {
                        to: Route::AdminContent {},
                        class: "text-sm px-3 py-2 text-gray-700 border border-gray-300 rounded hover:bg-gray-50 transition-colors",
                        "Back to Content"
                    }
                }
                button {
                    class: format!("text-sm px-3 py-2 text-white rounded transition-colors {}",
                        if saving() { "bg-gray-500 cursor-not-allowed" } else {
                            if is_edit_mode { "bg-blue-600 hover:bg-blue-700" } else { "bg-zinc-600 hover:bg-zinc-500" }
                        }
                    ),
                    disabled: saving(),
                    onclick: handle_save_blog_post,
                    if saving() {
                        if is_edit_mode { "Updating..." } else { "Creating..." }
                    } else {
                        if is_edit_mode { "Update Post" } else { "Create" }
                    }
                }
            }
        }

        div {
            class: "flex flex-col md:flex-row w-full gap-2",
            div {
                class: "flex w-full flex-col gap-2",
                div {
                    class: "flex-grow bg-white border rounded-md border-gray-200 p-4 min-h-36",
                    h2 { class: "text-lg font-medium mb-4", "Basic Information" }
                    div {
                        class: "flex flex-col gap-4 w-full",
                        CTextBox {
                            label: "Blog Title",
                            value: "{title}",
                            placeholder: "Enter blog post title...",
                            optional: false,
                            oninput: move |event: FormEvent| title.set(event.value())
                        }
                        CTextBox {
                            label: "Subtitle",
                            value: "{subtitle}",
                            placeholder: "Optional subtitle or summary...",
                            optional: true,
                            oninput: move |event: FormEvent| subtitle.set(event.value())
                        }
                        CTextBox {
                            label: "Thumbnail URL",
                            value: "{thumbnail_url}",
                            placeholder: "https://example.com/image.jpg",
                            optional: true,
                            oninput: move |event: FormEvent| thumbnail_url.set(event.value())
                        }
                    }
                },
                div {
                    class: "flex-grow bg-white border rounded-md border-gray-200 p-4 min-h-96",
                    h2 { class: "text-lg font-medium mb-4", "Blog Content" }
                    CTextArea {
                        label: "Blog Content (Markdown)",
                        placeholder: "Write your blog post content in markdown format...",
                        value: "{blog_md}",
                        optional: false,
                        class: "w-full h-96 font-mono text-sm p-3 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500 focus:border-blue-500 resize-y",
                        oninput: move |event: FormEvent| blog_md.set(event.value())
                    }
                }
            },
            div { class: "md:w-[38%] w-full min-w-0",
                div {
                    class: "bg-white border rounded-md border-gray-200 p-4 min-h-36 mb-2",
                    h2 { class: "text-lg font-medium mb-4", "Preview" }
                    if !title().trim().is_empty() {
                        div {
                            class: "border-l-4 border-blue-500 pl-4 mb-3",
                            h3 { class: "font-semibold text-gray-900 text-lg", "{title()}" }
                            if !subtitle().trim().is_empty() {
                                p { class: "text-gray-600 text-sm mt-1", "{subtitle()}" }
                            }
                        }
                    }
                    if !thumbnail_url().trim().is_empty() {
                        div {
                            class: "mb-3",
                            p { class: "text-sm text-gray-600 mb-2", "Thumbnail:" }
                            div {
                                class: "w-full h-32 bg-gray-100 rounded border flex items-center justify-center",
                                if thumbnail_url().trim().is_empty() {
                                    p { class: "text-gray-400 text-sm", "No thumbnail" }
                                } else {
                                    img {
                                        src: "{thumbnail_url()}",
                                        alt: "Thumbnail preview",
                                        class: "max-w-full max-h-full object-cover rounded",
                                    }
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
                },
                if is_edit_mode && loaded() {
                    if let Some(res) = blog_post_resource() {
                        if let Ok(blog_post) = res {
                            {
                                let posted_at_str = blog_post.posted_at.format("%Y-%m-%d %H:%M").to_string();
                                let updated_at_str = blog_post.updated_at.format("%Y-%m-%d %H:%M").to_string();
                                let post_id_str = blog_post.id.clone();

                                rsx! {
                                    div {
                                        class: "bg-white border rounded-md border-gray-200 p-4 min-h-36 mb-2",
                                        h2 { class: "text-lg font-medium mb-4", "Post Information" }
                                        div {
                                            class: "text-sm text-gray-600 space-y-2",
                                            p { "Posted: {posted_at_str}" }
                                            p { "Updated: {updated_at_str}" }
                                            p { "ID: {post_id_str}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
                div {
                    class: "bg-white border rounded-md border-gray-200 p-4 min-h-36 mb-2",
                    h2 { class: "text-lg font-medium mb-4", "Markdown Tips" }
                    div {
                        class: "text-sm text-gray-600 space-y-2",
                        p { "• Use # for main headings" }
                        p { "• Use ## for subheadings" }
                        p { "• **Bold text** with double asterisks" }
                        p { "• *Italic text* with single asterisks" }
                        p { "• Create lists with - or 1." }
                        p { "• Link text with [text](url)" }
                        p { "• Add images with ![alt](url)" }
                    }
                }
            }
        }
    }
}

#[component]
pub fn AdminCreateBlogPost() -> Element {
    AdminBlogPost(AdminBlogPostProps { id: None })
}

#[component]
pub fn AdminEditBlogPost(id: String) -> Element {
    let id_signal = use_signal(|| id);
    AdminBlogPost(AdminBlogPostProps { id: Some(id_signal) })
}
