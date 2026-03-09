#![allow(non_snake_case)] // Allow non-snake_case identifiers

use crate::Route;
use chrono::{Datelike, Local};
use dioxus::prelude::*;
use dioxus_i18n::{prelude::*, t};

#[component]
pub fn Footer() -> Element {
    let current_year = Local::now().year();

    let categories = vec![
        ("All.avif", t!("all-products"), "all"),
        ("Chondrogenic.avif", t!("chondrogenic"), "chondrogenic"),
        ("Osteogenic.avif", t!("osteogenic"), "osteogenic"),
        ("Protective.avif", t!("protective"), "protective"),
        ("Nootropic.avif", t!("nootropic"), "nootropic"),
        ("Other.avif", t!("other"), "other"),
    ];

    rsx! {

        footer {
            class: "bg-white border-t border-ui-border-base pt-10",

            div {
                class: "mx-auto w-full max-w-[1410px] p-4 py-6 lg:py-8",

                div {
                    class: "md:flex md:justify-between",

                    // Logo section
                    div {
                        class: "mb-6 md:mb-0",
                        Link {
                            to: Route::Home {},
                            class: "flex items-center",
                            img {
                                src: asset!("/assets/images/header.avif"),
                                alt: { t!("brand") },
                                class: "h-12"
                            }
                        }
                    }

                    // Links grid
                    div {
                        class: "grid grid-cols-2 gap-8 sm:gap-6 sm:grid-cols-2",

                        // Collections section
                        div {
                            h2 {
                                class: "mb-6 text-sm font-semibold text-gray-900",
                                { t!("collections") }
                            }
                            ul {
                                class: "text-gray-500 text-sm",
                                for (_, name, codename) in categories {
                                    li {
                                        class: "mb-4",
                                        Link {
                                            to: Route::Collection { codename: codename.to_string() },
                                            class: "hover:underline",
                                            { name }
                                        }
                                    }
                                }
                            }
                        }

                        // Follow Us section
                        /*
                        div {
                            h2 {
                                class: "mb-6 text-sm font-semibold text-gray-900",
                                { t!("follow-us") }
                            }
                            ul {
                                class: "text-gray-500 text-sm",
                                li {
                                    class: "mb-4",
                                    a {
                                        href: "https://twitter.com/",
                                        class: "hover:underline",
                                        { t!("twitter-bio") }
                                    }
                                }
                                li {
                                    class: "",
                                    a {
                                        href: "https://t.me/",
                                        class: "hover:underline",
                                        { t!("telegram") }
                                    }
                                }
                            }
                        }
                        */

                        // Important section
                        div {
                            h2 {
                                class: "mb-6 text-sm font-semibold text-gray-900",
                                { t!("important") }
                            }
                            ul {
                                class: "text-gray-500 text-sm",
                                li {
                                    class: "mb-4",
                                    Link {
                                        to: Route::Policies {},
                                        class: "hover:underline",
                                        { t!("policies") }
                                    }
                                }
                                li {
                                    class: "mb-4",
                                    Link {
                                        to: Route::Contact {},
                                        class: "hover:underline",
                                        { t!("contact") }
                                    }
                                }
                                li {
                                    class: "",
                                    Link {
                                        to: Route::Faq {},
                                        class: "hover:underline",
                                        { t!("faq") }
                                    }
                                }
                            }
                        }
                    }
                }

                // Divider
                hr {
                    class: "my-6 border-gray-200 sm:mx-auto lg:my-8"
                }

                // Bottom section
                div {
                    class: "sm:flex sm:items-center sm:justify-between",

                    span {
                        class: "text-sm text-gray-500 sm:text-center",
                        "© {current_year} "
                        Link {
                            to: Route::Home {},
                            class: "hover:underline",
                            { t!("brand") }
                        }
                        ". "
                        { t!("all-rights-reserved") }
                    }

                    /*
                    div {
                        class: "flex mt-4 sm:justify-center sm:mt-0",
                        div {
                            class: "flex justify-center",
                            a {
                                target: "_blank",
                                rel: "norefer",
                                title: t!("trustpilot-reviews"),
                                href: "https://www.trustpilot.com/review/noveldist.com",
                                img {
                                    style: "height:40px;",
                                    src: asset!("/assets/images/trustpilot.avif")
                                }
                            }
                        }
                    }
                    */
                }
            }
        }
    }
}
