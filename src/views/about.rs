use crate::Route;
use dioxus::prelude::*;
use dioxus_i18n::t;

const ABOUT_CSS: Asset = asset!("/assets/styling/about.css");

#[component]
pub fn About() -> Element {
    let svg_content = r##"
        <svg class="absolute">
            <filter id="turbulence" x="0" y="0" width="100%" height="100%">
                <feTurbulence id="sea-filter" numOctaves="3" seed="2" baseFrequency="0.02 0.05"></feTurbulence>
                <feDisplacementMap scale="10" in="SourceGraphic"></feDisplacementMap>
                <animate xlink:href="#sea-filter" attributeName="baseFrequency" dur="60s" keyTimes="0;0.5;1" values="0.02 0.06;0.04 0.08;0.02 0.06" repeatCount="indefinite"></animate>
            </filter>
        </svg>
    "##;

    rsx! {
        document::Title { { format!("{} - {}", t!("brand"), t!("about-us") ) } }
        document::Link { rel: "stylesheet", href: ABOUT_CSS }

        div {
            class: "bg-[#08080a] w-full px-5 md:px-6 text-gray-100 py-4 pt-6 md:pt-12 flex justify-center",
            div {
                class: "max-w-[1000px] w-full",
                h2 { class: "mb-4", { t!("about-us") } }
                p { { t!("about-line-1") } }
                p {
                    class: "mt-2",
                    { t!("about-line-2") }
                }
                p {
                    class: "mt-2",
                    { t!("about-line-3") }
                }
                p {
                    class: "mt-2",
                    { t!("about-line-4") }
                }
                p {
                    class: "mt-2",
                    { t!("about-line-5") }
                }
                p {
                    class: "mt-2",
                    { t!("about-affiliate-line") }
                }
                div {
                    class: "mt-5 flex",
                    a {
                        title: { t!("x-twitter") },
                        target: "_blank",
                        href: "https://x.com/penchantbio",
                        img {
                            class: "invert h-6",
                            src: asset!("/assets/icons/x-logo.svg")
                        }
                    }
                    a {
                        class: "ml-4",
                        title: "Novel Distributions Telegram",
                        target: "_blank",
                        href: "https://t.me/",
                        img {
                            class: "invert h-6",
                            src: asset!("/assets/icons/telegram-logo.svg")
                        }
                    }
                }
                div {
                    class: "flex mt-5 overflow-x-auto hidden",
                    Link {
                        to: Route::Faq {},
                        class: "mr-2",
                        button {
                            class: "px-4 py-2 min-w-32 text-sm border-gray-500 hover:bg-gray-900 border rounded-md",
                            { t!("faq") }
                        }
                    }
                    Link {
                        to: Route::Contact {},
                        class: "mr-2",
                        button {
                            class: "px-4 py-2 min-w-32 text-sm border-gray-500 hover:bg-gray-900 border rounded-md",
                            { t!("contact-us") }
                        }
                    }
                    Link {
                        to: Route::Policies {},
                        class: "mr-2",
                        button {
                            class: "px-4 py-2 min-w-32 text-sm border-gray-500 hover:bg-gray-900 border rounded-md",
                            { t!("policies") }
                        }
                    }
                    Link {
                        to: Route::ShippingPolicy {},
                        class: "mr-2",
                        button {
                            class: "px-4 py-2 min-w-32 text-sm border-gray-500 hover:bg-gray-900 border rounded-md",
                            { t!("shipping") }
                        }
                    }
                }
            }
        }

        div {
            class: "flex justify-center w-full bg-[#08080a]",
            div {
                class: "water-background w-[700px] md:max-w-[1000px] bg-[#08080a]",
                div { class: "water", style: { format!("background-image: url({});", asset!("/assets/images/cherry-blossom.jpg")) } }
            }
        }

        div {
            dangerous_inner_html: svg_content
        }

        div {
            class: "h-48 w-full bg-[#08080a]"
        }
    }
}
