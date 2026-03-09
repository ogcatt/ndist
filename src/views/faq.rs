use crate::Route;
use dioxus::prelude::*;
use dioxus_i18n::t;

#[derive(Clone, PartialEq)]
struct FaqItem {
    question: String,
    answer: String,
}

#[component]
pub fn Faq() -> Element {
    let faqs_payments = use_memo(move || {
        vec![
            FaqItem {
                question: t!("faq-pay-q1"),
                answer: t!("faq-pay-a1"),
            },
            FaqItem {
                question: t!("faq-pay-q2"),
                answer: t!("faq-pay-a2"),
            },
            FaqItem {
                question: t!("faq-pay-q3"),
                answer: t!("faq-pay-a3"),
            },
            FaqItem {
                question: t!("faq-pay-q4"),
                answer: t!("faq-pay-a4"),
            },
            FaqItem {
                question: t!("faq-pay-q5"),
                answer: t!("faq-pay-a5"),
            },
        ]
    });

    let faqs_products = use_memo(move || {
        vec![
            FaqItem {
                question: t!("faq-prod-q4"),
                answer: t!("faq-prod-a4"),
            },
            FaqItem {
                question: t!("faq-prod-q5"),
                answer: t!("faq-prod-a5"),
            },
            FaqItem {
                question: t!("faq-prod-q6"),
                answer: t!("faq-prod-a6"),
            },
            FaqItem {
                question: t!("faq-prod-q8"),
                answer: t!("faq-prod-a8"),
            },
            FaqItem {
                question: t!("faq-prod-q9"),
                answer: t!("faq-prod-a9"),
            },
        ]
    });

    let faqs_misc = use_memo(move || {
        vec![
            FaqItem {
                question: t!("faq-misc-q1"),
                answer: t!("faq-misc-a1"),
            },
            FaqItem {
                question: t!("faq-misc-q2"),
                answer: t!("faq-misc-a2"),
            },
            FaqItem {
                question: t!("faq-misc-q3"),
                answer: t!("faq-misc-a3"),
            },
            FaqItem {
                question: t!("faq-misc-q5"),
                answer: t!("faq-misc-a5"),
            },
        ]
    });

    rsx! {
        document::Title { { format!("{} - {}", t!("brand"), t!("frequently-asked-questions") ) } }

        div { class: "py-6 md:py-12 flex justify-center",
            div { class: "max-w-[1000px] w-full px-4",
                h2 { class: "mb-6", { t!("frequently-asked-questions") } }

                // Payments Section
                h3 { class: "text-xl mb-4", { t!("payments") } }
                div { class: "bg-white rounded-lg border mb-5 overflow-hidden",
                    for (index, faq) in faqs_payments().iter().enumerate() {
                        FaqCollapse {
                            key: "payment_{index}",
                            question: faq.question.clone(),
                            answer: faq.answer.clone(),
                            is_last: index == faqs_payments().len() - 1
                        }
                    }
                }

                // Products Section
                h3 { class: "text-xl mb-4", { t!("products") } }
                div { class: "bg-white rounded-lg border mb-5 overflow-hidden",
                    for (index, faq) in faqs_products().iter().enumerate() {
                        FaqCollapse {
                            key: "product_{index}",
                            question: faq.question.clone(),
                            answer: faq.answer.clone(),
                            is_last: index == faqs_products().len() - 1
                        }
                    }
                }

                // Miscellaneous Section
                h3 { class: "text-xl mb-4", { t!("miscellaneous") } }
                div { class: "bg-white rounded-lg shadow-sm border mb-5",
                    for (index, faq) in faqs_misc().iter().enumerate() {
                        FaqCollapse {
                            key: "misc_{index}",
                            question: faq.question.clone(),
                            answer: faq.answer.clone(),
                            is_last: index == faqs_misc().len() - 1
                        }
                    }
                }

                // Shipping Section
                h3 { class: "text-xl mb-4", { t!("shipping") } }
                p { class: "text-gray-600",
                    { format!("{} ", t!("you-can-visit")) }
                    Link { to: Route::ShippingPolicy {}, class: "text-blue-600 hover:text-blue-800 underline",
                        { t!("shipping-page-l") }
                    }
                    { format!(" {}", t!("for-more-information")) }
                }
            }
        }
    }
}

#[component]
fn FaqCollapse(question: String, answer: String, is_last: bool) -> Element {
    let mut is_open = use_signal(|| false);

    let border_class = if is_last { "" } else { "border-b" };

    rsx! {
        div { class: "{border_class}",
            button {
                class: "w-full flex items-center justify-between pl-3 pr-2 min-h-9 text-left hover:bg-gray-50 transition-colors duration-150",
                onclick: move |_| is_open.toggle(),
                div { class: "flex-1 flex items-center",
                    div { class: "text-gray-400 mr-2", "Q:" }
                    span { class: "text-gray-800", "{question}" }
                }
                div { class: "ml-2 text-gray-500 transition-transform duration-200",
                    class: if is_open() { "transform rotate-180" } else { "" },
                    svg {
                        width: "1.5em",
                        height: "1.5em",
                        view_box: "0 0 24 24",
                        class: "",
                        role: "presentation",
                        path {
                            d: "M7.41,8.58L12,13.17L16.59,8.58L18,10L12,16L6,10L7.41,8.58Z",
                            fill: "currentColor",
                            class: "svelte-uen7q8"
                        }
                    }
                }
            }
            div {
                class: "overflow-hidden transition-all duration-300 ease-in-out",
                style: if is_open() {
                    "max-height: 1000px; opacity: 1;"
                } else {
                    "max-height: 0; opacity: 0;"
                },
                div {
                    class: "px-3 py-2 text-gray-600 border-t text-sm",
                    dangerous_inner_html: answer
                }
            }
        }
    }
}
