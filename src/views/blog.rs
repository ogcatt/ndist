use crate::Route;
use crate::backend::cache::use_cached_server;
use crate::backend::front_entities::BlogPost;
use crate::backend::server_functions;
use dioxus::prelude::*;
use dioxus_i18n::t;
use std::time::Duration;

#[component]
pub fn BlogPostPage(id: ReadOnlySignal<String>) -> Element {
    let mut current_post = use_signal(|| None::<BlogPost>);
    let mut blog_not_found = use_signal(|| false);
    let blog_posts_req = use_cached_server(
        "blog_posts_list",
        || server_functions::get_blog_posts(),
        Duration::from_secs(300),
    );

    use_effect(move || {
        let blog_posts_result = blog_posts_req();
        match blog_posts_result {
            Some(Ok(blog_posts)) => {
                if let Some(post) = blog_posts.iter().find(|p| p.id == id()) {
                    current_post.set(Some(post.clone()));
                    blog_not_found.set(false);
                } else {
                    current_post.set(None);
                    blog_not_found.set(true);
                }
            }
            Some(Err(_)) => {
                current_post.set(None);
                blog_not_found.set(true);
            }
            None => {
                current_post.set(None);
            }
        }
    });

    rsx! {
        div {
            class: "py-6 md:py-12 flex justify-center",
            div {
                class: "max-w-[1000px] w-full px-5",
                div {
                    id: "blog",
                    if blog_not_found() {
                        div {
                            class: "text-center py-12",
                            h2 {
                                class: "text-gray-900 mb-4",
                                "Blog Post Not Found"
                            }
                            p {
                                class: "text-gray-500",
                                "The blog post you're looking for doesn't exist."
                            }
                        }
                    } else if let Some(post) = current_post() {
                        article {
                            class: "max-w-none",
                            if let Some(thumbnail_url) = &post.thumbnail_url {
                                img {
                                    src: "{thumbnail_url}",
                                    alt: "{post.title}",
                                    class: "w-full h-64 object-cover rounded-lg mb-8"
                                }
                            }
                            h2 { class: "mb-4", "{post.title}" },
                            if let Some(subtitle) = &post.subtitle {
                                p {
                                    class: "text-xl text-gray-600 mb-6",
                                    "{subtitle}"
                                }
                            }
                            p {
                                class: "text-gray-400 text-sm mb-8",
                                "Posted on {post.posted_at.format(\"%B %d, %Y\")}"
                            }
                            div {
                                class: " max-w-none",
                                dangerous_inner_html: "{post.blog_md}"
                            }
                        }
                    } else {
                        /*
                        div {
                            class: "text-center py-12",
                            p {
                                class: "text-gray-500",
                                "Loading blog post..."
                            }
                        }
                        */
                    }
                }
            }
        }
    }
}

pub fn BlogPosts() -> Element {
    let blog_posts_req = use_cached_server(
        "blog_posts_list",
        || server_functions::get_blog_posts(),
        Duration::from_secs(300),
    );

    rsx! {
        div {
            class: "py-6 md:py-12 flex justify-center",
            div {
                class: "max-w-[1000px] w-full px-5",
                div {
                    id: "blog-posts",
                    h2 { class: "mb-6", { t!("blog") } },

                    {
                        match &*blog_posts_req.read() {
                            Some(Ok(blog_posts)) => rsx! {
                                if blog_posts.is_empty() {
                                    div {
                                        class: "text-center py-12",
                                        p {
                                            class: "text-gray-500 text-lg",
                                            "No blog posts available yet."
                                        }
                                    }
                                } else {
                                    div {
                                        class: "grid gap-8 md:grid-cols-2 lg:grid-cols-1",
                                        for post in blog_posts {
                                            article {
                                                key: "{post.id}",
                                                class: "bg-white rounded-lg shadow-sm border border-gray-200 overflow-hidden hover:shadow-md transition-shadow duration-200",
                                                Link {
                                                    to: Route::BlogPostPage { id: post.id.clone() },
                                                    class: "block",
                                                    div {
                                                        class: "flex flex-col md:flex-row",
                                                        if let Some(thumbnail_url) = &post.thumbnail_url {
                                                            div {
                                                                class: "md:w-1/3 flex-shrink-0",
                                                                img {
                                                                    src: "{thumbnail_url}",
                                                                    alt: "{post.title}",
                                                                    class: "w-full h-48 md:h-full object-cover"
                                                                }
                                                            }
                                                        }
                                                        div {
                                                            class: "flex-1 p-6",
                                                            div {
                                                                h2 {
                                                                    class: "text-2xl text-gray-900 mb-2 hover:text-blue-600 transition-colors",
                                                                    "{post.title}"
                                                                }
                                                                if let Some(subtitle) = &post.subtitle {
                                                                    p {
                                                                        class: "text-lg text-gray-600 mb-3",
                                                                        "{subtitle}"
                                                                    }
                                                                }
                                                                div {
                                                                    class: "flex items-center justify-between",
                                                                    p {
                                                                        class: "text-gray-400 text-sm",
                                                                        "{post.posted_at.format(\"%B %d, %Y\")}"
                                                                    }
                                                                    span {
                                                                        class: "text-blue-600 text-sm font-medium hover:text-blue-700",
                                                                        "Read more →"
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
                                div {
                                    class: "text-center py-12",
                                    p {
                                        class: "text-red-500 mb-4",
                                        "Failed to load blog posts"
                                    }
                                    button {
                                        class: "bg-blue-600 text-white px-4 py-2 rounded hover:bg-blue-700 transition-colors",
                                        onclick: move |_| {
                                            // Force refresh by invalidating cache
                                            // This would depend on your cache implementation
                                        },
                                        "Try Again"
                                    }
                                }
                            },
                            None => rsx! {}
                        }
                    }
                }
            }
        }
    }
}
