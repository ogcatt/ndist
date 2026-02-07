use crate::Route;
use dioxus::prelude::*;
use dioxus_i18n::t;
use js_sys::eval;

fn copy_to_clipboard(text: String) {
    let _ = eval(&format!(
        r#"navigator.clipboard.writeText('{}').then(() => console.log('Copied to clipboard'))"#,
        text.replace("'", "\\'")
    ));
}

#[component]
pub fn Contact() -> Element {
    let mut manual = use_signal(|| false);

    let copy_email_to_clipboard = move |_| {
        manual.set(true);

        // Construct email to prevent web scraping
        let email = format!("{}@{}.{}", "support", "noveldist", "com");
        copy_to_clipboard(email);
    };

    rsx! {
        document::Title { {t!("contact-info")} }

        div {
            class: "content-container py-6 md:py-12 flex justify-center",
            div {
                class: "max-w-[1000px] w-full",
                h2 {
                    class: "mb-4",
                    {t!("contact-us")}
                }

                p {
                    class: "text-ui-fg-subtle",
                    {t!("contact-support-email")}
                }

                p {
                    class: "text-ui-fg-subtle flex text-lg my-3 unselectable cursor-pointer",
                    onmousedown: copy_email_to_clipboard,
                    "support "
                    img {
                        src: asset!("/assets/icons/at-symbol.svg"),
                        class: "h-[18px] px-1 mt-[6px] opacity-[65%]"
                    }
                    " noveldist.com"
                }

                if manual() {
                    p {
                        class: "text-ui-fg-muted text-xs mt-1 mb-3",
                        {t!("email-copied")}
                    }
                }

                p {
                    class: "text-ui-fg-subtle mt-2",
                    {t!("contact-twitter-prefix")}
                    " "
                    a {
                        class: "a",
                        href: "https://twitter.com/PenchantBio",
                        target: "_blank",
                        {t!("twitter-x")}
                    }
                    {t!("contact-twitter-suffix")}
                }

                p {
                    class: "text-ui-fg-subtle mt-2",
                    {t!("response-time-info")}
                }

                p {
                    class: "text-ui-fg-subtle mt-2",
                    {t!("check-faq-prefix")}
                    " "
                    Link {
                        class: "a",
                        to: Route::Faq {},
                        {t!("faq")}
                    }
                    " "
                    {t!("check-faq-suffix")}
                }
            }
        }
    }
}
