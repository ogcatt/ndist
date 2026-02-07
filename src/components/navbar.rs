#![allow(non_snake_case)] // Allow non-snake_case identifiers

use crate::Route;
use chrono::{Datelike, Local};
use dioxus::prelude::*;
use dioxus_i18n::{prelude::*, t};
use std::rc::Rc;
use std::str::FromStr;

use crate::backend::server_functions::{get_or_create_basket, get_products};
use crate::i18n::{Language, use_language_setter};
use crate::utils::{GLOBAL_CART, countries::*, filter_products};

use crate::components::{LanguagePopup, SearchResults};

#[component]
pub fn Header() -> Element {
    // Mobile menu state signals
    let mut open_menu = use_signal(|| false);
    let mut mobile_categories_open = use_signal(|| false);
    let mut mobile_about_open = use_signal(|| false);
    let mut mobile_research_open = use_signal(|| false);
    let mut mobile_languages_open = use_signal(|| false);

    // Location dropdown state
    let mut location_dropdown_open = use_signal(|| false);

    let mut search_bar_open = use_signal(|| false);

    // Language functionality
    let mut language_setter = use_language_setter();

    let mut cart_total_quantity = use_signal(|| 0i32);

    // Updated date checks to match svelte version
    let now = Local::now();
    let is_christmas = now.month() == 12 && (now.day() >= 18 && now.day() <= 31);
    let is_new_year = now.month() == 1 && now.day() <= 3;
    let is_ind_day = now.month() == 7 && now.day() == 4;

    let mut cart_resource = use_resource(move || async move {
        get_or_create_basket().await
    });

    // When cart_resource resolves, set GLOBAL_CART and quantity
    use_effect(move || {
        if let Some(cart_res) = cart_resource() {
            match cart_res {
                Ok(basket) => {
                    GLOBAL_CART.with_mut(|c| *c = Some(basket.clone()));

                    // Update quantity immediately
                    let total = basket
                        .items
                        .as_ref()
                        .map(|items| items.iter().map(|i| i.quantity).sum())
                        .unwrap_or(0);
                    cart_total_quantity.set(total);
                }
                Err(e) => {
                    GLOBAL_CART.with_mut(|c| *c = None);
                    cart_total_quantity.set(0);
                    tracing::error!("Failed to load basket: {:?}", e);
                }
            }
        }
    });

    // Keep cart_total_quantity in sync with GLOBAL_CART (for updates elsewhere)
    use_effect(move || {
        let total = GLOBAL_CART
            .read()
            .as_ref()
            .and_then(|cart| cart.items.as_ref())
            .map(|items| items.iter().map(|i| i.quantity).sum())
            .unwrap_or(0);
        cart_total_quantity.set(total);
    });

    // Update your handle_language_change function
    let mut handle_language_change = move |lang_code: &str| {
        if let Ok(language) = Language::from_str(lang_code) {
            // Save to storage first
            if let Err(e) = crate::i18n::set_user_language(language.clone()) {
                tracing::error!("Failed to save language preference: {}", e);
            }

            // Update the signal
            language_setter.set(Some(language));

            // Close dropdowns
            location_dropdown_open.set(false);

            tracing::info!("Language changed to: {}", lang_code);
        }
    };

    let mut input_ref = use_signal(|| None::<Rc<MountedData>>);
    let mut search_query = use_signal(|| String::new());

    // Use an effect to focus when search_bar_open changes
    use_effect(move || {
        if *search_bar_open.read() {
            if let Some(input) = input_ref.read().as_ref() {
                let _ = input.set_focus(true);
            }
        }
    });

    // Create a trigger signal
    let mut trigger_search = use_signal(|| false);

    // Use a resource that only runs when triggered
    let products_data = use_resource(move || async move {
        if trigger_search() {
            get_products().await
        } else {
            // Return empty result or default state when not triggered
            Ok(Vec::new()) // Adjust this based on your expected return type
        }
    });

    // Use an effect to trigger the resource when search bar opens
    use_effect(move || {
        if *search_bar_open.read() {
            trigger_search.set(true);
        }
    });

    // Add this effect to prevent body scrolling when menu is open
    use_effect(move || {
        let document = web_sys::window().unwrap().document().unwrap();
        let body = document.body().unwrap();

        // Cast to Element to access style methods
        let body_element: &web_sys::Element = body.as_ref();

        if *open_menu.read() {
            // Prevent body scroll when menu is open
            let _ = body_element.set_attribute("style", "overflow: hidden");
        } else {
            // Restore body scroll when menu is closed
            let _ = body_element.remove_attribute("style");
        }
    });

    rsx! {

        LanguagePopup {}

        if *search_bar_open.read() {
            div {
                class: "fixed inset-0 bg-black bg-opacity-20 z-40",
                onclick: move |_| {
                    search_bar_open.set(false);
                    search_query.set(String::new());
                }
            }

            div {
                class: "absolute top-16 left-0 h-20 z-50 bg-gray-100 w-full shadow-lg flex items-center justify-center px-4",
                input {
                    class: "w-full max-w-md px-4 py-2 text-black bg-gray-100 border-none focus:outline-none placeholder-gray-500",
                    placeholder: { t!("search-for-products") },
                    r#type: "text",
                    value: "{search_query.read()}",
                    oninput: move |evt| {
                        search_query.set(evt.value());
                    },
                    onmounted: move |mounted| {
                        *input_ref.write() = Some(mounted.data());
                    }
                }
            }

            if !search_query.read().is_empty() {
                if let Some(Ok(products)) = products_data.read().as_ref() {
                    SearchResults {
                        products: filter_products(products, &search_query.read()),
                        search_query: search_query.read().clone(),
                        on_product_click: move |_| {
                            search_query.set(String::new()); // Clear search
                            search_bar_open.set(false); // Close search bar
                        }
                    }
                }
            }
        }

        div {
            class: "sticky top-0 inset-x-0 z-50",
            header {
                class: "relative h-18 mx-auto border-b duration-200 bg-white border-ui-border-base",
                nav {
                    class: "content-container txt-xsmall-plus text-ui-fg-subtle flex items-center justify-between w-full h-full text-smm md:text-sm",
                    // Left section: Mobile toggle and desktop menus
                    div {
                        class: "flex-1 basis-0 h-full flex items-center",
                        // Mobile menu toggle
                        div {
                            class: "h-full flex items-center md:hidden",
                            div {
                                onclick: move |_| {
                                    open_menu.set(!open_menu());
                                },
                                class: "flex items-center justify-center cursor-pointer",
                                img {
                                    src: asset!("/assets/icons/menu-outline.svg"),
                                    alt: "Menu",
                                    class: "h-[26px]",
                                }
                            }
                        },
                        // Desktop navigation - fixed container structure

                        div {
                            class: "h-full hidden md:flex",
                            // Products dropdown
                            div {
                                class: "h-full relative group",
                                Link {
                                    to: Route::Collections { },
                                    class: "h-full",
                                    button {
                                        class: "mr-5 relative text-nowrap h-full flex items-center transition-all ease-out duration-200 hover:text-ui-fg-base",
                                        title: t!("browse-products"),
                                        { t!("products") },
                                        span {
                                            style: "margin-top:2px;",
                                            class: "pl-1.5",
                                            img {
                                                src: asset!("/assets/icons/down-arrow.svg"),
                                                alt: "",
                                                width: "11"
                                            }
                                        }
                                    }
                                },
                                div {
                                    class: "absolute w-max max-w-[250px] bg-white rounded-b-md shadow-lg z-10 hidden group-hover:block border border-gray-200 border-t-0",
                                    Link {
                                        to: Route::Collection { codename: String::from("all") },
                                        class: "block px-5 py-3 text-sm text-gray-700 hover:bg-gray-100 transition-colors duration-200 ease-out",
                                        { t!("all-products") }
                                    },
                                    Link {
                                        to: Route::Collection { codename: String::from("nootropics") },
                                        class: "block px-5 py-3 text-sm text-gray-700 hover:bg-gray-100 transition-colors duration-200 ease-out",
                                        { t!("nootropics") }
                                    },
                                    Link {
                                        to: Route::Collection { codename: String::from("pbios") },
                                        class: "block px-5 py-3 text-sm text-gray-700 hover:bg-gray-100 transition-colors duration-200 ease-out",
                                        { t!("pbios") }
                                    },
                                    Link {
                                        to: Route::Collection { codename: String::from("peptides-and-longevity") },
                                        class: "block px-5 py-3 text-sm text-gray-700 hover:bg-gray-100 transition-colors duration-200 ease-out",
                                        { t!("peptides-and-longevity") }
                                    },
                                    Link {
                                        to: Route::Collection { codename: String::from("natural") },
                                        class: "block px-5 py-3 text-sm text-gray-700 hover:bg-gray-100 transition-colors duration-200 ease-out",
                                        { t!("natural") }
                                    },
                                    Link {
                                        to: Route::Collection { codename: String::from("sarms-and-physical") },
                                        class: "block px-5 py-3 text-sm text-gray-700 hover:bg-gray-100 transition-colors duration-200 ease-out",
                                        { t!("sarms-and-physical") }
                                    },
                                    /*
                                    hr {},
                                    Link {
                                        to: Route::Collection { codename: String::from("bodygen") },
                                        class: "block px-5 py-3 text-sm text-gray-700 hover:bg-gray-100 bg-cyan-50 transition-colors duration-200 ease-out",
                                        { format!("{}{}", t!("bodygen"), t!("products")) }
                                    },
                                    Link {
                                        to: Route::Collection { codename: String::from("pheroblend") },
                                        class: "block px-5 py-3 text-sm text-gray-700 hover:bg-gray-100 bg-cyan-50 transition-colors duration-200 ease-out",
                                        { format!("{}{}", t!("pheroblend"), t!("products")) }
                                    }
                                    */
                                }
                            },
                            // About Us dropdown
                            div {
                                class: "h-full relative group",
                                Link {
                                    to: Route::About {},
                                    class: "h-full",
                                    button {
                                        class: "mr-5 relative text-nowrap h-full flex items-center transition-all ease-out duration-200 hover:text-ui-fg-base",
                                        title: format!("{} {}", t!("about"),  t!("brand")),
                                        { t!("about-us") },
                                        span {
                                            style: "margin-top:2px;",
                                            class: "pl-1.5",
                                            img {
                                                src: asset!("/assets/icons/down-arrow.svg"),
                                                alt: "",
                                                width: "11"
                                            }
                                        }
                                    }
                                },
                                div {
                                    class: "absolute w-max max-w-[250px] bg-white rounded-b-md shadow-lg z-10 hidden group-hover:block border border-gray-200 border-t-0",
                                    Link {
                                        to: Route::Faq {},
                                        class: "block px-5 py-3 text-sm text-gray-700 hover:bg-gray-100 transition-colors duration-200 ease-out",
                                        title: t!("frequently-asked-questions"),
                                        div {
                                            class: "flex pr-0.5 items-center",
                                            img {
                                                class: "blende",
                                                src: asset!("/assets/icons/help-circle-outline.svg"),
                                                style: "height:20px;"
                                            },
                                            div {
                                                class: "ml-2",
                                                { t!("faq") }
                                            }
                                        }
                                    },
                                    Link {
                                        to: Route::BlogPosts { },
                                        class: "block px-5 py-3 text-sm text-gray-700 hover:bg-gray-100 transition-colors duration-200 ease-out",
                                        title: t!("blog"),
                                        div {
                                            class: "flex pr-0.5 items-center",
                                            img {
                                                class: "blende",
                                                src: asset!("/assets/icons/newspaper-outline.svg"),
                                                style: "height:20px;"
                                            },
                                            div {
                                                class: "ml-2",
                                                { t!("blog") }
                                            }
                                        }
                                    },
                                    Link {
                                        to: Route::Contact {},
                                        class: "block px-5 py-3 text-sm text-gray-700 hover:bg-gray-100 transition-colors duration-200 ease-out",
                                        title: t!("contact-info"),
                                        div {
                                            class: "flex pr-0.5 items-center",
                                            img {
                                                class: "blende",
                                                src: asset!("/assets/icons/mail-outline.svg"),
                                                style: "height:20px;"
                                            },
                                            div {
                                                class: "ml-2",
                                                { t!("contact-us") }
                                            }
                                        }
                                    },
                                    Link {
                                        to: Route::ShippingPolicy { },
                                        class: "block px-5 py-3 text-sm text-gray-700 hover:bg-gray-100 transition-colors duration-200 ease-out",
                                        title: t!("shipping-info"),
                                        div {
                                            class: "flex pr-0.5 items-center",
                                            img {
                                                class: "blende",
                                                src: asset!("/assets/icons/airplane-outline.svg"),
                                                style: "height:20px;"
                                            },
                                            div {
                                                class: "ml-2",
                                                { t!("shipping") }
                                            }
                                        }
                                    },
                                    Link {
                                        to: Route::Policies {},
                                        class: "block px-5 py-3 text-sm text-gray-700 hover:bg-gray-100 transition-colors duration-200 ease-out",
                                        title: t!("store-policies"),
                                        div {
                                            class: "flex pr-0.5 items-center",
                                            img {
                                                class: "blende",
                                                src: asset!("/assets/icons/document-text-outline.svg"),
                                                style: "height:20px;"
                                            },
                                            div {
                                                class: "ml-2",
                                                { t!("policies") }
                                            }
                                        }
                                    }
                                }
                            },
                            div {
                                class: "h-full relative group",
                                button {
                                    class: "mr-5 relative text-nowrap h-full flex items-center transition-all ease-out duration-200 hover:text-ui-fg-base cursor-default",
                                    title: t!("research-and-writeups"),
                                    { t!("research") },
                                    span {
                                        style: "margin-top:2px;",
                                        class: "pl-1.5",
                                        img {
                                            src: asset!("/assets/icons/down-arrow.svg"),
                                            alt: "",
                                            width: "11"
                                        }
                                    }
                                },
                                div {
                                    class: "absolute w-max max-w-[250px] bg-white rounded-b-md shadow-lg z-10 hidden group-hover:block border border-gray-200 border-t-0",
                                    a {
                                        href: "https://labs.penchant.bio",
                                        target: "_blank",
                                        class: "block px-5 py-3 text-sm text-gray-700 hover:bg-gray-100 transition-colors duration-200 ease-out",
                                        title: t!("visit-in-new-tab", name: { t!("brand-labs") }),
                                        div {
                                            class: "flex justify-center pr-0.5 items-center",
                                            img {
                                                src: asset!("/assets/images/plabs-wbg.avif"),
                                                style: "height:24px;"
                                            },
                                            div {
                                                class: "pl-2",
                                                { t!("brand-labs") }
                                            },
                                            img {
                                                class: "blende ml-2 self-center",
                                                src: asset!("/assets/icons/open-outline.svg"),
                                                style: "height:18px;"
                                            }
                                        }
                                    }
                                }
                            },
                            // Peptide Calculator link (no dropdown)
                            Link {
                                to: Route::PeptideCalculator {},
                                class: "h-full",
                                button {
                                    class: "relative text-nowrap h-full flex items-center transition-all ease-out duration-200 hover:text-ui-fg-base",
                                    title: t!("peptide-calculator-info"),
                                    { t!("peptide-calculator") }
                                }
                            }
                        }
                    },
                    // Center section: Logo
                    div {
                        class: "flex items-center h-full",
                        Link {
                            to: Route::Home {},
                            class: "pl-2 fadeyy",
                            if is_ind_day {
                                img {
                                    src: asset!("/assets/images/header-4th.avif"),
                                    alt: t!("brand").to_uppercase(),
                                    title: t!("independence-day"),
                                    class: "h-8 md:h-12"
                                }
                            } else if is_christmas {
                                img {
                                    src: asset!("/assets/images/header-christmas.avif"),
                                    alt: t!("brand").to_uppercase(),
                                    title: t!("merry-christmas"),
                                    class: "h-8 md:h-12"
                                }
                            } else if is_new_year {
                                img {
                                    src: asset!("/assets/images/header.avif"),
                                    alt: t!("brand").to_uppercase(),
                                    title: t!("new-year"),
                                    class: "h-10 md:h-11 lg:h-12"
                                }
                            } else {
                                img {
                                    src: asset!("/assets/images/header.avif"),
                                    alt: t!("brand").to_uppercase(),
                                    class: "h-10 md:h-11 lg:h-12"
                                }
                            }
                        }
                    },
                    // Right section: Location, Account, Cart
                    div {
                        class: "flex items-center gap-x-6 h-full flex-1 basis-0 justify-end",
                        // Location button with dropdown (desktop only)
                        div {
                            class: "md:block hidden h-full z-5 relative",
                            div {
                                class: "relative h-full",
                                button {
                                    onclick: move |_| {
                                        location_dropdown_open.set(!location_dropdown_open());
                                    },
                                    class: "h-full pointer",
                                    title: format!("{}", { t!("your-location") }),
                                    div {
                                        class: "flex justify-center",
                                        img {
                                            class: "fadey",
                                            //src: asset!("/assets/icons/location-sharp.svg"),
                                            src: asset!("/assets/icons/language-outline.svg"),
                                            style: "height:24px;margin-bottom:-1px;"
                                        }
                                    }
                                },
                                // Location/Language dropdown
                                if *location_dropdown_open.read() {
                                    div {
                                        class: "absolute max-h-96 overflow-y-auto right-0 top-full w-max max-w-[280px] bg-white rounded-b-md shadow-lg z-10 border border-gray-200",
                                        div {
                                            class: "p-3 border-b border-gray-100",
                                            div {
                                                class: "text-sm font-semibold text-gray-900 mb-1",
                                                "Language / Region"
                                            },
                                            div {
                                                class: "text-xs text-gray-500",
                                                "Select your preferred language"
                                            }
                                        },
                                        div {
                                            class: "py-1",
                                            for language_option in LANGUAGE_OPTIONS.iter() {
                                                button {
                                                    onclick: move |_| handle_language_change(language_option.code),
                                                    class: "w-full flex items-center px-4 py-3 text-sm text-gray-700 hover:bg-gray-100 transition-colors duration-200 ease-out",
                                                    div {
                                                        class: "flex items-center flex-1",
                                                        div {
                                                            class: "text-lg mr-3",
                                                            "{language_option.flag}"
                                                        },
                                                        div {
                                                            class: "text-left",
                                                            div {
                                                                class: "font-medium text-gray-900",
                                                                "{language_option.name}"
                                                            },
                                                            div {
                                                                class: "text-xs text-gray-500",
                                                                "{language_option.country}"
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
                        // Search button
                        div {
                            class: "md:block hidden h-full z-8",
                            div {
                                class: "relative h-full cursor-pointer",
                                title: t!("search-products"),
                                button {
                                    class: "h-full",
                                    aria_label: "Search Products",
                                    onclick: move |_| {
                                        search_bar_open.set(!search_bar_open());
                                    },
                                    div {
                                        class: "flex justify-center",
                                        img {
                                            class: "fadey",
                                            src: asset!("/assets/icons/search-circle-outline.svg"),
                                            style: "height:29px;"
                                        }
                                    }
                                }
                            }
                        },


                        // Account button (desktop only)

                        /*
                        div {
                            class: "md:block hidden h-full z-8",
                            div {
                                class: "relative h-full",
                                a {
                                    href: "/admin/dashboard",
                                    title: format!("{}/{}", { t!("account") }, { t!("dashboard") }),
                                    button {
                                        class: "h-full",
                                        aria_label: "Dashboard",
                                        div {
                                            class: "flex justify-center",
                                            img {
                                                class: "fadey",
                                                src: asset!("/assets/icons/person-circle-outline.svg"),
                                                style: "height:27px;"
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        */
                        // Cart button
                        div {
                            class: "h-full z-8",
                            div {
                                class: "relative h-full",
                                Link {
                                    to: Route::Cart {},
                                    title: format!("{} (? {})", { t!("cart") }, { t!("items").to_lowercase() }),
                                    button {
                                        class: "h-full",
                                        aria_label: "Cart",
                                        div {
                                            class: "flex justify-center items-center",
                                            img {
                                                alt: "Cart",
                                                src: asset!("/assets/icons/bag-outline.svg"),
                                                style: "height:24px;"
                                            },
                                            div {
                                                class: "bg-black text-white text-center flex items-center justify-center ml-[-6px] mt-[-10px]",
                                                style: "border-radius: 50%; width: 16px; height: 16px; font-size: 10px; line-height: 1;",
                                                "{cart_total_quantity}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            },
            // Mobile menu with updated structure
            if *open_menu.read() {
                div {
                    class: "absolute mobile-menu top-18 w-full border-ui-border-base border-b shadow-md block md:hidden bg-white",
                    style: "z-index: 400; height: calc(100dvh - 72px); max-height: calc(100dvh - 72px);",

                    div {
                        class: "flex flex-col h-full",

                        // Scrollable content area
                        div {
                            class: "flex-1 overflow-y-auto",

                            // Main menu items
                            ul {
                                class: "flex flex-col",

                                // Search option at the top
                                li {
                                    button {
                                        onclick: move |_| {
                                            search_bar_open.set(!search_bar_open());
                                            open_menu.set(false);
                                        },
                                        class: "w-full text-left py-3 px-4 flex items-center text-gray-900 hover:bg-gray-100 transition-colors duration-200 ease-out border-b border-gray-100",
                                        img {
                                            class: "blende mr-3",
                                            src: asset!("/assets/icons/search-outline.svg"),
                                            style: "height:20px;"
                                        },
                                        { t!("search") }
                                    }
                                },

                                // Home link
                                li {
                                    Link {
                                        to: Route::Home {},
                                        onclick: move |_| open_menu.set(false),
                                        class: "block py-3 px-4 text-gray-900 hover:bg-gray-100 transition-colors duration-200 ease-out border-b border-gray-100",
                                        { t!("home") }
                                    }
                                },

                                // Products section
                                li {
                                    button {
                                        onclick: move |_| {
                                            mobile_categories_open.set(!mobile_categories_open());
                                        },
                                        class: "w-full text-left py-3 px-4 flex text-gray-900 hover:bg-gray-100 transition-colors duration-200 ease-out border-b border-gray-100",
                                        { t!("products") },
                                        span {
                                            class: "pl-2 self-center ml-auto",
                                            img {
                                                src: asset!("/assets/icons/down-arrow.svg"),
                                                alt: "",
                                                width: "12"
                                            }
                                        }
                                    },
                                    if *mobile_categories_open.read() {
                                        div {
                                            class: "bg-gray-50 border-b border-gray-100",
                                            Link {
                                                to: Route::Collection { codename: String::from("all") },
                                                onclick: move |_| open_menu.set(false),
                                                class: "block py-2 px-6 text-gray-700 hover:bg-gray-100 transition-colors duration-200 ease-out border-b border-gray-200",
                                                { t!("all-products") }
                                            },
                                            Link {
                                                to: Route::Collection { codename: String::from("pbios") },
                                                onclick: move |_| open_menu.set(false),
                                                class: "block py-2 px-6 text-gray-700 hover:bg-gray-100 transition-colors duration-200 ease-out border-b border-gray-200",
                                                { t!("pbios") }
                                            },
                                            Link {
                                                to: Route::Collection { codename: String::from("nootropics") },
                                                onclick: move |_| open_menu.set(false),
                                                class: "block py-2 px-6 text-gray-700 hover:bg-gray-100 transition-colors duration-200 ease-out border-b border-gray-200",
                                                { t!("nootropics") }
                                            },
                                            Link {
                                                to: Route::Collection { codename: String::from("peptides-and-longevity") },
                                                onclick: move |_| open_menu.set(false),
                                                class: "block py-2 px-6 text-gray-700 hover:bg-gray-100 transition-colors duration-200 ease-out border-b border-gray-200",
                                                { t!("peptides-and-longevity") }
                                            },
                                            Link {
                                                to: Route::Collection { codename: String::from("natural") },
                                                onclick: move |_| open_menu.set(false),
                                                class: "block py-2 px-6 text-gray-700 hover:bg-gray-100 transition-colors duration-200 ease-out border-b border-gray-200",
                                                { t!("natural") }
                                            },
                                            Link {
                                                to: Route::Collection { codename: String::from("physical") },
                                                onclick: move |_| open_menu.set(false),
                                                class: "block py-2 px-6 text-gray-700 hover:bg-gray-100 transition-colors duration-200 ease-out border-b border-gray-200",
                                                { t!("sarms-and-physical") }
                                            }
                                        }
                                    }
                                },

                                // Info section
                                li {
                                    button {
                                        onclick: move |_| {
                                            mobile_about_open.set(!mobile_about_open());
                                        },
                                        class: "w-full text-left py-3 px-4 flex text-gray-900 hover:bg-gray-100 transition-colors duration-200 ease-out border-b border-gray-100",
                                        { t!("info") },
                                        span {
                                            class: "pl-2 self-center ml-auto",
                                            img {
                                                src: asset!("/assets/icons/down-arrow.svg"),
                                                alt: "",
                                                width: "12"
                                            }
                                        }
                                    },
                                    if *mobile_about_open.read() {
                                        div {
                                            class: "bg-gray-50 border-b border-gray-100",
                                            Link {
                                                to: Route::About {},
                                                onclick: move |_| open_menu.set(false),
                                                class: "block py-2 px-6 flex items-center text-gray-700 hover:bg-gray-100 transition-colors duration-200 ease-out border-b border-gray-200",
                                                img {
                                                    class: "blende mr-2",
                                                    src: asset!("/assets/images/logo-black.avif"),
                                                    style: "height:20px;"
                                                },
                                                { t!("about-us") }
                                            },
                                            Link {
                                                to: Route::Faq {},
                                                onclick: move |_| open_menu.set(false),
                                                class: "block py-2 px-6 flex items-center text-gray-700 hover:bg-gray-100 transition-colors duration-200 ease-out border-b border-gray-200",
                                                img {
                                                    class: "blende mr-2",
                                                    src: asset!("/assets/icons/help-circle-outline.svg"),
                                                    style: "height:20px;"
                                                },
                                                { t!("faq") }
                                            },
                                            Link {
                                                to: Route::BlogPosts {},
                                                onclick: move |_| open_menu.set(false),
                                                class: "block py-2 px-6 flex items-center text-gray-700 hover:bg-gray-100 transition-colors duration-200 ease-out border-b border-gray-200",
                                                img {
                                                    class: "blende mr-2",
                                                    src: asset!("/assets/icons/newspaper-outline.svg"),
                                                    style: "height:20px;"
                                                },
                                                { t!("blog") }
                                            },
                                            Link {
                                                to: Route::ShippingPolicy {},
                                                onclick: move |_| open_menu.set(false),
                                                class: "block py-2 px-6 flex items-center text-gray-700 hover:bg-gray-100 transition-colors duration-200 ease-out border-b border-gray-200",
                                                img {
                                                    class: "blende mr-2",
                                                    src: asset!("/assets/icons/airplane-outline.svg"),
                                                    style: "height:20px;"
                                                },
                                                { t!("shipping") }
                                            },
                                            Link {
                                                to: Route::Policies {},
                                                onclick: move |_| open_menu.set(false),
                                                class: "block py-2 px-6 flex items-center text-gray-700 hover:bg-gray-100 transition-colors duration-200 ease-out border-b border-gray-200",
                                                img {
                                                    class: "blende mr-2",
                                                    src: asset!("/assets/icons/document-text-outline.svg"),
                                                    style: "height:20px;"
                                                },
                                                { t!("policies") }
                                            }
                                        }
                                    }
                                },

                                // Research section
                                li {
                                    button {
                                        onclick: move |_| {
                                            mobile_research_open.set(!mobile_research_open());
                                        },
                                        class: "w-full text-left py-3 px-4 flex text-gray-900 hover:bg-gray-100 transition-colors duration-200 ease-out border-b border-gray-100",
                                        { t!("research") },
                                        span {
                                            class: "pl-2 self-center ml-auto",
                                            img {
                                                src: asset!("/assets/icons/down-arrow.svg"),
                                                width: "12"
                                            }
                                        }
                                    },
                                    if *mobile_research_open.read() {
                                        div {
                                            class: "bg-gray-50 border-b border-gray-100",
                                            a {
                                                href: "https://labs.penchant.bio",
                                                target: "_blank",
                                                onclick: move |_| open_menu.set(false),
                                                class: "block py-2 px-6 flex items-center text-gray-700 hover:bg-gray-100 transition-colors duration-200 ease-out border-b border-gray-200",
                                                img {
                                                    class: "blende mr-2",
                                                    src: asset!("/assets/images/plabs-wbg.avif"),
                                                    style: "height:20px;"
                                                },
                                                { t!("brand-labs") },
                                                img {
                                                    class: "blende ml-2",
                                                    src: asset!("/assets/icons/open-outline.svg"),
                                                    style: "height:18px;"
                                                }
                                            }
                                        }
                                    }
                                },

                                // Peptide Calculator link
                                li {
                                    Link {
                                        to: Route::PeptideCalculator {},
                                        onclick: move |_| open_menu.set(false),
                                        class: "block py-3 px-4 text-gray-900 hover:bg-gray-100 transition-colors duration-200 ease-out border-b border-gray-100",
                                        { t!("peptide-calculator") }
                                    }
                                }
                            }
                        },

                        // Bottom section - Languages (collapsible)
                        div {
                            class: "border-t-2 border-gray-200 bg-gray-50 flex-shrink-0",

                            // Language toggle button
                            button {
                                onclick: move |_| {
                                    mobile_languages_open.set(!mobile_languages_open());
                                },
                                class: "w-full px-4 py-3 flex items-center text-gray-900 hover:bg-gray-100 transition-colors duration-200 ease-out",
                                img {
                                    class: "blende mr-3",
                                    src: asset!("/assets/icons/language-outline.svg"),
                                    style: "height:20px;"
                                },
                                span {
                                    class: "text-sm font-semibold flex-1 text-left",
                                    { t!("languages") }
                                },
                                span {
                                    class: "ml-auto",
                                    img {
                                        src: asset!("/assets/icons/down-arrow.svg"),
                                        alt: "",
                                        width: "12",
                                        style: if *mobile_languages_open.read() { "" } else { "transform: rotate(180deg);" }
                                    }
                                }
                            },

                            // Language options (collapsible with its own scroll)
                            if *mobile_languages_open.read() {
                                div {
                                    class: "max-h-48 overflow-y-auto bg-white border-t border-gray-200",
                                    for language_option in LANGUAGE_OPTIONS.iter() {
                                        button {
                                            onclick: move |_| {
                                                handle_language_change(language_option.code);
                                                open_menu.set(false);
                                            },
                                            class: "w-full flex items-center p-3 text-gray-700 hover:bg-gray-100 transition-colors duration-200 ease-out border-b border-gray-100",
                                            div {
                                                class: "flex items-center flex-1",
                                                div {
                                                    class: "text-lg mr-3",
                                                    "{language_option.flag}"
                                                },
                                                div {
                                                    class: "text-left",
                                                    div {
                                                        class: "font-medium text-gray-900 text-sm",
                                                        "{language_option.name}"
                                                    },
                                                    div {
                                                        class: "text-xs text-gray-500",
                                                        "{language_option.country}"
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
    }
}
