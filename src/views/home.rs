// src/views/home.rs (don't remove this line)
use dioxus::prelude::*;
use dioxus_i18n::t;
use std::time::Duration;

use crate::Route;
use crate::backend::cache::{use_cached_server, use_hybrid_cache};
use crate::backend::server_functions;
use crate::components::{CollectionsGrid, Meta, ProductCard, WideProductCard}; // Import the ProductCard component
use crate::utils::{countries::allowed_countries, sort_products_by_priority};
// use crate::components::Collections; // Uncomment if you have a Collections component

/// The Home page component that will be rendered when the current route is `[Route::Home]`
#[component]
pub fn Home() -> Element {
    // Use our caching hook
    let products_data = use_hybrid_cache(
        "get_products", // Unique key for this server function
        || async { server_functions::get_products().await },
        Duration::from_secs(180), // Cache
    );

    rsx! {
        // Include seo/meta tags
        Meta {}

        div {
            class: "pt-8 md:pt-12",
            div {
                class: "content-container py-0 sm:pt-0 sm:pb-12 px-6",


                div {
                    class: "mb-8 flex flex-col justify-center w-full border-ui-border-base border rounded-md overflow-hidden",

                    // Image container with reduced height
                    div {
                        class: "flex justify-center lg:min-h-60 max-h-80 overflow-hidden",
                        img {
                            src: asset!("/assets/images/blank-banner.jpg"),
                            alt: "Banner",
                            class: "w-full max-h-80 h-full",
                            style: "object-fit: cover;",
                            draggable: "false",
                            decoding: "async",
                            "fetchpriority": "high"
                        }
                    }

                    // Text bar with icon
                    div {
                        class: "p-4 bg-gray-50 flex items-center gap-3",
                        img {
                            class: "blende",
                            src: asset!("/assets/icons/newspaper-outline.svg"),
                            style: "height:20px;"
                        },
                        Link {
                            to: Route::BlogPostPage { id: "5456dcfc-ff47-4fe6-b53a-7f631bec1719".to_string() },
                            class: "text-sm text-gray-700 underline",
                            "Visit our blog post for details about our return and what's in store."
                        }
                    }
                }

                {
                    let filtered_products = match &*products_data.read() {
                        Some(products) => {
                            let mut pre_order_products: Vec<_> = products
                                .iter()
                                .filter(|product| product.pre_order == true)
                                .cloned()
                                .collect();
                            pre_order_products.sort_by(|a, b| b.created_at.cmp(&a.created_at));
                            pre_order_products.into_iter().take(2).collect()
                        },
                        None => Vec::new()
                    };

                    if !filtered_products.is_empty() {
                        rsx! {
                            div {
                                class: "flex justify-between mb-8",
                                p {
                                    class: "font-normal font-sans txt-medium text-xl",
                                    { t!("pre-orders") }
                                }
                            }
                            ul {
                                class: "mb-16 grid md:grid-cols-2 lg:grid-cols-2 grid-cols-1 gap-x-4 md:gap-x-6 md:gap-y-20 gap-y-10",
                                for product in filtered_products {
                                    li {
                                        class: "",
                                        WideProductCard {
                                            key: "{product.id}",
                                            product: product.clone(),
                                            alert: true,
                                            top_class: ""
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        rsx! {}
                    }
                }

                // Header section with title and view all link
                div {
                    class: "flex justify-between mb-8",
                    p {
                        class: "font-normal font-sans txt-medium text-xl",
                        { t!("featured-products") }
                    }
                    Link {
                        class: "flex gap-x-1 items-center group",
                        to: Route::Collection { codename: String::from("all") },
                        p {
                            class: "font-normal font-sans txt-medium text-ui-fg-interactive",
                            { t!("view-all") }
                        }
                        svg {
                            xmlns: "http://www.w3.org/2000/svg",
                            width: "20",
                            height: "20",
                            fill: "none",
                            class: "group-hover:rotate-45 ease-in-out duration-200",
                            path {
                                stroke: "var(--text-ui-fg-interactive)",
                                "stroke-linecap": "round",
                                "stroke-linejoin": "round",
                                "stroke-width": "1.5",
                                d: "m5.75 14.25 8.5-8.5m0 0h-7.5m7.5 0v7.5"
                            }
                        }
                    }
                }

                {match &*products_data.read() {
                    Some(products) => rsx! {

                        ul {
                            class: "grid md:grid-cols-3 lg:grid-cols-4 grid-cols-2 gap-x-4 md:gap-x-6 md:gap-y-20 gap-y-10",

                            for product in sort_products_by_priority(&products) {
                                li {
                                    class: "",
                                    ProductCard {
                                        key: "{product.id}",
                                        product: product.clone(),
                                        top_class: ""
                                    }
                                }
                            }

                        }

                        // Bottom text section
                        /*
                        div {
                            class: "text-center mt-16",
                            p {
                                class: "text-ui-fg-muted text-sm",
                                { t!("twitter-plug") } {" "}
                                a {
                                    class: "a",
                                    target: "_blank",
                                    href: "https://x.com/",
                                    { t!("x-twitter") }
                                }
                                "."
                            }
                        }
                        */

                        // Mobile-only Categories section
                        div {
                            class: "block md:hidden md:mt-16 mb-10",
                            div {
                                class: "md:hidden flex mb-8 mt-8 justify-between",
                                p {
                                    class: "font-normal font-sans txt-medium text-xl",
                                    { t!("categories") }
                                }
                            }
                            CollectionsGrid {}
                        }
                    },
                    /*
                    Some(err) => rsx! {
                        div {
                            class: "text-center py-8",
                            p {
                                class: "text-red-500",
                                { t!("error-loading-products", error: format!("{:?}", err)) }
                            }
                        }
                    },
                    */
                    None => rsx! {
                        ul {
                            class: "grid md:grid-cols-3 lg:grid-cols-4 grid-cols-2 gap-x-4 md:gap-x-6 md:gap-y-20 gap-y-10",
                            for _num in 0..4 {
                                li {
                                    class: "",
                                    ProductCard {
                                        loading: true
                                    }
                                }
                            }
                        }
                    }
                }}
            }

            // Labs and shipping sections
            div {
                class: "w-full bg-black mt-12 md:mt-8",
                div {
                    class: "w-full content-container py-0 pt-10 pb-6 px-6",

                    // About Penchant Labs section
                    /*
                    div {
                        class: "bg-graddy mt-12 mb-12 md:flex md:justify-center w-full border-ui-border-base border rounded-md md:max-h-80 overflow-hidden",
                        div {
                            class: "w-full md:w-2/5",
                            img {
                                src: asset!("/assets/images/plabs.avif"),
                                style: "object-fit: cover;",
                                class: "w-full max-h-80 h-full",
                                draggable: "false",
                                decoding: "async",
                                loading: "lazy",
                                alt: "Promo"
                            }
                        }
                        div {
                            class: "md:w-3/5 w-full p-5 md:p-8 text-sm lg:text-base overflow-y-scroll",
                            h2 {
                                class: "mb-4 font-weight-450 text-xl",
                                div {
                                    class: "flex justify-start",
                                    img {
                                        style: "height:24px;",
                                        alt: "Info-Icon",
                                        src: asset!("/assets/icons/information-circle-outline.svg")
                                    }
                                    div {
                                        class: "ml-1",
                                        style: "margin-top:-4px;",
                                        { t!("about-penchant-labs") }
                                    }
                                }
                            }
                            p {
                                class: "text-ui-fg-subtle",
                                { t!("labs-description-1") }
                            }
                            p {
                                class: "mt-2 text-ui-fg-subtle",
                                { t!("labs-description-2") }
                            }
                            p {
                                class: "mt-2 text-ui-fg-subtle",
                                { t!("labs-description-3") }
                            }
                            div {
                                class: "mt-5",
                                a {
                                    href: "https://labs.penchant.bio/library",
                                    target: "_blank",
                                    class: "inline-flex items-center px-4 py-2 bg-black text-ui-fg-on-inverted rounded-md hover:bg-ui-bg-interactive-hover transition-colors",
                                    { t!("visit-penchant-labs") }
                                }
                            }
                        }
                    }
                    */

                    // Shipping worldwide section
                    div {
                        class: "my-32 text-gray-100",
                        div {
                            class: "flex justify-center pb-4",
                            div {
                                img {
                                    style: "box-shadow: 0px 0px 20px 10px gray;",
                                    class: "h-48 invert rounded-[50%]",
                                    alt: "Earth Globe",
                                    src: asset!("/assets/images/earth-globe-with-continents-maps.svg")
                                }
                            }
                        }
                        div {
                            class: "text-2xl flex items-center justify-center mt-8 text-center",
                            p { { t!("shipping-worldwide") } }
                        }
                        div {
                            class: "flex justify-center text-center mt-8",
                            div {
                                p {
                                    class: "text-gray-100 text-sm flex",
                                    img {
                                        class: "invert mr-2",
                                        style: "height:24px;",
                                        alt: "Airplane Icon",
                                        src: asset!("/assets/icons/airplane-outline.svg")
                                    }
                                    { t!("shipping-countries", num: allowed_countries().len()) }
                                }
                                p {
                                    class: "text-gray-100 text-sm flex mt-2",
                                    img {
                                        class: "invert mr-2",
                                        style: "height:24px;",
                                        alt: "Alarm Icon",
                                        src: asset!("/assets/icons/alarm-outline.svg")
                                    }
                                    { t!("shipping-time") }
                                }
                                p {
                                    class: "text-gray-100 text-sm flex mt-2",
                                    img {
                                        class: "invert mr-2",
                                        style: "height:24px;",
                                        alt: "Help Circle Outline",
                                        src: asset!("/assets/icons/help-circle-outline.svg")
                                    }
                                    { t!("shipping-support") }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
