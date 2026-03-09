#![allow(non_snake_case)] // Allow non-snake_case identifiers

use crate::Route;
use chrono::{Datelike, Local};
use dioxus::prelude::*;
use dioxus_i18n::{prelude::*, t};
use std::rc::Rc;
use std::str::FromStr;
use std::time::Duration;

use crate::backend::server_functions::{self, get_or_create_basket, get_products};
use crate::backend::cache::{use_hybrid_cache, use_stale_while_revalidate};
use crate::utils::{GLOBAL_CART, countries::*, filter_products};

use crate::components::{AccountButton, AccountMobileButton, AccountPopupProvider, SearchResults};

const NAVBAR_CSS: Asset = asset!("/assets/styling/navbar.css");

#[component]
pub fn Header() -> Element {
    // Mobile menu state signals
    let mut open_menu = use_signal(|| false);
    let mut mobile_categories_open = use_signal(|| false);
    let mut mobile_about_open = use_signal(|| false);
    let mut mobile_groups_open = use_signal(|| false);
    let mut mobile_research_open = use_signal(|| false);

    let mut search_bar_open = use_signal(|| false);

    let mut cart_total_quantity = use_signal(|| 0i32);

    // Track which group is being hovered for the nested dropdown
    let mut hovered_group_id = use_signal(|| None::<String>);

    // Get session info using cache
    let session_info = use_stale_while_revalidate(
        "get_session_info",
        || async { server_functions::get_session_info().await },
        Duration::from_secs(60),
    );

    // Get groups data using hybrid cache (only if user has groups)
    let has_groups = session_info
        .read()
        .as_ref()
        .map(|info| !info.group_ids.is_empty())
        .unwrap_or(false);

    let groups_data = use_stale_while_revalidate(
        "get_user_groups",
        || async { server_functions::get_user_groups().await },
        Duration::from_secs(180),
    );

    // Get all products for filtering by group access
    let all_products_data = use_hybrid_cache(
        "get_products",
        || async { server_functions::get_products().await },
        Duration::from_secs(180),
    );

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
        document::Link { rel: "stylesheet", href: NAVBAR_CSS }
        AccountPopupProvider {
            // Search bar overlay
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
                                search_query.set(String::new());
                                search_bar_open.set(false);
                            }
                        }
                    }
                }
            }

            div {
                class: "navbar-dark sticky top-0 inset-x-0 z-50",
                header {
                    class: "relative h-18 mx-auto duration-200 bg-white border-ui-border-base",
                    nav {
                        class: "txt-xsmall-plus text-ui-fg-subtle flex items-center justify-between w-full h-full text-smm md:text-sm px-4 md:px-6",
                    // Left section: Logo and Desktop navigation
                    div {
                        class: "flex items-center h-full gap-x-6",
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
                        // Logo (left side on desktop, center on mobile handled below)
                        div {
                            class: "hidden md:flex items-center h-full",
                            Link {
                                to: Route::Home {},
                                class: "fadeyy",
                                img {
                                    src: asset!("/assets/images/header.avif"),
                                    alt: t!("brand").to_uppercase(),
                                    class: "h-10 md:h-11 lg:h-12"
                                }
                            }
                        },
                        // Desktop navigation dropdowns
                        div {
                            class: "h-full hidden md:flex",
                            // Products dropdown
                            div {
                                class: "h-full relative group",
                                button {
                                    class: "mr-5 relative text-nowrap h-full flex items-center transition-all ease-out duration-200 hover:text-ui-fg-base cursor-default",
                                    title: t!("categories"),
                                    { t!("categories") },
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
                                    Link {
                                        to: Route::Collection { codename: String::from("all") },
                                        class: "block px-5 py-3 text-sm text-gray-700 hover:bg-gray-100 transition-colors duration-200 ease-out",
                                        { t!("all-products") }
                                    },
                                    Link {
                                        to: Route::Collection { codename: String::from("chondrogenic") },
                                        class: "block px-5 py-3 text-sm text-gray-700 hover:bg-gray-100 transition-colors duration-200 ease-out",
                                        { t!("chondrogenic") }
                                    },
                                    Link {
                                        to: Route::Collection { codename: String::from("osteogenic") },
                                        class: "block px-5 py-3 text-sm text-gray-700 hover:bg-gray-100 transition-colors duration-200 ease-out",
                                        { t!("osteogenic") }
                                    },
                                    Link {
                                        to: Route::Collection { codename: String::from("protective") },
                                        class: "block px-5 py-3 text-sm text-gray-700 hover:bg-gray-100 transition-colors duration-200 ease-out",
                                        { t!("protective") }
                                    },
                                    Link {
                                        to: Route::Collection { codename: String::from("nootropic") },
                                        class: "block px-5 py-3 text-sm text-gray-700 hover:bg-gray-100 transition-colors duration-200 ease-out",
                                        { t!("nootropic") }
                                    },
                                    Link {
                                        to: Route::Collection { codename: String::from("other") },
                                        class: "block px-5 py-3 text-sm text-gray-700 hover:bg-gray-100 transition-colors duration-200 ease-out",
                                        { t!("other") }
                                    },
                                }
                            },
                            // Groups dropdown (only show if user has groups)
                            if has_groups {
                                div {
                                    class: "h-full relative group",
                                    button {
                                        class: "mr-5 relative text-nowrap h-full flex items-center transition-all ease-out duration-200 hover:text-ui-fg-base cursor-default",
                                        title: t!("my-groups"),
                                        { t!("groups") },
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
                                        class: "absolute w-max min-w-[250px] bg-white rounded-b-md shadow-lg z-10 hidden group-hover:block border border-gray-200 border-t-0",
                                        {
                                            // Clone data to avoid holding borrows across closures
                                            let groups_opt = groups_data.read().clone();
                                            let session_opt = session_info.read().clone();

                                            if let Some(groups) = groups_opt {
                                                if let Some(session) = session_opt {
                                                    // Filter groups to only show those the user is a member of
                                                    let user_groups: Vec<_> = groups.iter()
                                                        .filter(|g| session.group_ids.contains(&g.id))
                                                        .cloned()
                                                        .collect();

                                                    rsx! {
                                                        for group in user_groups {
                                                            div {
                                                                class: "relative group/item",
                                                                onmouseenter: move |_| {
                                                                    hovered_group_id.set(Some(group.id.clone()));
                                                                },
                                                                onmouseleave: move |_| {
                                                                    hovered_group_id.set(None);
                                                                },
                                                                Link {
                                                                    to: Route::GroupPage { id: group.id.clone() },
                                                                    class: "block px-5 py-3 text-sm text-gray-700 hover:bg-gray-100 transition-colors duration-200 ease-out",
                                                                    div {
                                                                        class: "font-medium",
                                                                        "{group.name}"
                                                                    },
                                                                    if let Some(description) = &group.description {
                                                                        div {
                                                                            class: "text-xs text-gray-500 mt-1 line-clamp-2",
                                                                            "{description}"
                                                                        }
                                                                    }
                                                                },
                                                                // Nested dropdown for products in this group
                                                                if hovered_group_id.read().as_ref() == Some(&group.id) {
                                                                    {
                                                                        // Clone products to avoid holding borrow
                                                                        let products_opt = all_products_data.read().clone();
                                                                        if let Some(products) = products_opt {
                                                                            let group_id = group.id.clone();
                                                                            let group_products: Vec<_> = products.iter()
                                                                                .filter(|p| {
                                                                                    p.access_groups.as_ref()
                                                                                        .map(|groups| groups.contains(&group_id))
                                                                                        .unwrap_or(false)
                                                                                })
                                                                                .cloned()
                                                                                .collect();

                                                                            if !group_products.is_empty() {
                                                                                rsx! {
                                                                                    div {
                                                                                        class: "absolute left-full top-0 ml-0 w-80 bg-white rounded-r-md shadow-lg border border-gray-200 border-l-0 max-h-96 overflow-y-auto",
                                                                                        for product in group_products {
                                                                                            Link {
                                                                                                to: Route::ProductPage { handle: product.handle.clone() },
                                                                                                class: "block px-4 py-3 hover:bg-gray-50 transition-colors duration-150 border-b border-gray-100 last:border-b-0",
                                                                                                div {
                                                                                                    class: "font-medium text-gray-900 text-sm",
                                                                                                    {
                                                                                                        if product.title.contains("(") {
                                                                                                            product.title.clone()
                                                                                                        } else {
                                                                                                            format!(
                                                                                                                "{} ({})",
                                                                                                                product.title,
                                                                                                                product.product_form.to_frontend_string()
                                                                                                            )
                                                                                                        }
                                                                                                    }
                                                                                                },
                                                                                                if let Some(subtitle) = &product.subtitle {
                                                                                                    div {
                                                                                                        class: "text-xs text-gray-500 mt-1",
                                                                                                        "{subtitle}"
                                                                                                    }
                                                                                                } else if let Some(variants) = &product.variants {
                                                                                                    div {
                                                                                                        class: "text-xs text-gray-500 mt-1",
                                                                                                        for (i, variant) in variants.iter().enumerate() {
                                                                                                            if i != 0 { ", " }
                                                                                                            "{variant.variant_name}"
                                                                                                        }
                                                                                                    }
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                }
                                                                            } else {
                                                                                rsx! { }
                                                                            }
                                                                        } else {
                                                                            rsx! { }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                } else {
                                                    rsx! { }
                                                }
                                            } else {
                                                rsx! { }
                                            }
                                        }
                                    }
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
                                class: "h-full relative group hidden",
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
                            }
                        }
                    },
                    // Center section: Logo (mobile only)
                    div {
                        class: "flex md:hidden items-center h-full absolute left-1/2 transform -translate-x-1/2",
                        Link {
                            to: Route::Home {},
                            class: "pl-2 fadeyy",
                            img {
                                src: asset!("/assets/images/header.avif"),
                                alt: t!("brand").to_uppercase(),
                                class: "h-8"
                            }
                        }
                    },
                    // Right section: Account, Search, Cart icons
                    div {
                        class: "flex items-center gap-x-6 h-full",
                        // Account button (desktop only)
                        div {
                            class: "md:block hidden h-full z-8",
                            div {
                                class: "relative h-full",
                                AccountButton {}
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
                        // Admin button (desktop only, admin users only)
                        if session_info.read().as_ref().map(|info| info.admin).unwrap_or(false) {
                            div {
                                class: "md:block hidden h-full z-8",
                                div {
                                    class: "relative h-full",
                                    Link {
                                        to: Route::AdminDashboard {},
                                        title: "Admin Dashboard",
                                        button {
                                            class: "h-full",
                                            aria_label: "Admin Dashboard",
                                            div {
                                                class: "flex justify-center",
                                                img {
                                                    class: "fadey",
                                                    src: asset!("/assets/icons/color-wand-outline.svg"),
                                                    style: "height:27px;"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        },
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
            }

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
                                        { t!("categories") },
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
                                                to: Route::Collection { codename: String::from("chondrogenic") },
                                                onclick: move |_| open_menu.set(false),
                                                class: "block py-2 px-6 text-gray-700 hover:bg-gray-100 transition-colors duration-200 ease-out border-b border-gray-200",
                                                { t!("chondrogenic") }
                                            },
                                            Link {
                                                to: Route::Collection { codename: String::from("osteogenic") },
                                                onclick: move |_| open_menu.set(false),
                                                class: "block py-2 px-6 text-gray-700 hover:bg-gray-100 transition-colors duration-200 ease-out border-b border-gray-200",
                                                { t!("osteogenic") }
                                            },
                                            Link {
                                                to: Route::Collection { codename: String::from("protective") },
                                                onclick: move |_| open_menu.set(false),
                                                class: "block py-2 px-6 text-gray-700 hover:bg-gray-100 transition-colors duration-200 ease-out border-b border-gray-200",
                                                { t!("protective") }
                                            },
                                            Link {
                                                to: Route::Collection { codename: String::from("nootropic") },
                                                onclick: move |_| open_menu.set(false),
                                                class: "block py-2 px-6 text-gray-700 hover:bg-gray-100 transition-colors duration-200 ease-out border-b border-gray-200",
                                                { t!("nootropic") }
                                            },
                                            Link {
                                                to: Route::Collection { codename: String::from("other") },
                                                onclick: move |_| open_menu.set(false),
                                                class: "block py-2 px-6 text-gray-700 hover:bg-gray-100 transition-colors duration-200 ease-out border-b border-gray-200",
                                                { t!("other") }
                                            }
                                        }
                                    }
                                },

                                // Groups section (mobile - only show if user has groups)
                                if has_groups {
                                    li {
                                        button {
                                            onclick: move |_| {
                                                mobile_groups_open.set(!mobile_groups_open());
                                            },
                                            class: "w-full text-left py-3 px-4 flex text-gray-900 hover:bg-gray-100 transition-colors duration-200 ease-out border-b border-gray-100",
                                            { t!("groups") },
                                            span {
                                                class: "pl-2 self-center ml-auto",
                                                img {
                                                    src: asset!("/assets/icons/down-arrow.svg"),
                                                    alt: "",
                                                    width: "12"
                                                }
                                            }
                                        },
                                        if *mobile_groups_open.read() {
                                            div {
                                                class: "bg-gray-50 border-b border-gray-100",
                                                if let Some(groups) = groups_data.read().as_ref() {
                                                    if let Some(session) = session_info.read().as_ref() {
                                                        {
                                                            let user_groups: Vec<_> = groups.iter()
                                                                .filter(|g| session.group_ids.contains(&g.id))
                                                                .collect();

                                                            rsx! {
                                                                for group in user_groups {
                                                                    Link {
                                                                        to: Route::GroupPage { id: group.id.clone() },
                                                                        onclick: move |_| open_menu.set(false),
                                                                        class: "block py-3 px-6 text-gray-700 hover:bg-gray-100 transition-colors duration-200 ease-out border-b border-gray-200",
                                                                        div {
                                                                            class: "font-medium",
                                                                            "{group.name}"
                                                                        },
                                                                        if let Some(description) = &group.description {
                                                                            div {
                                                                                class: "text-xs text-gray-500 mt-1",
                                                                                "{description}"
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
                                        class: "hidden w-full text-left py-3 px-4 flex text-gray-900 hover:bg-gray-100 transition-colors duration-200 ease-out border-b border-gray-100",
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
                                }
                            }
                        }

                        // Bottom section - Account
                        div {
                            class: "border-t-2 border-gray-200 bg-gray-50 flex-shrink-0",
                            // Admin link (mobile only, admin users only)
                            if session_info.read().as_ref().map(|info| info.admin).unwrap_or(false) {
                                Link {
                                    to: Route::AdminDashboard {},
                                    onclick: move |_| open_menu.set(false),
                                    class: "flex items-center py-3 px-4 text-gray-900 hover:bg-gray-100 transition-colors duration-200 ease-out border-b border-gray-200",
                                    img {
                                        class: "blende mr-3",
                                        src: asset!("/assets/icons/color-wand-outline.svg"),
                                        style: "height:20px;"
                                    },
                                    "Admin"
                                }
                            }
                            AccountMobileButton {}
                        }
                    }
                    }
                }
            }
        },
    }
}
