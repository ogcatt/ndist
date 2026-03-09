use dioxus::prelude::*;
#[cfg(feature = "server")]
use dotenvy::dotenv;
#[cfg(feature = "server")]
use std::env;
#[cfg(feature = "server")]
use tokio;
#[cfg(feature = "server")]
use tower_http::services::ServeDir;
#[cfg(feature = "server")]
use axum::{
    response::{IntoResponse, Response},
    http::{header, StatusCode},
};
#[cfg(feature = "server")]
use crate::backend::{
    auth::{auth_middleware, setup_app_state},
    payments,
    sitemap::generate_sitemap,
    meta_cache,
    meta_injection::inject_meta_middleware,
    api::invite::generate_invite_code_handler,
};
use crate::i18n::{
    config,
    consts::{STORAGE_STATE, STORAGE_TARGET},
    use_user_preferred_language,
};
use components::{AdminWrapper, CheckoutHeader, HeaderFooter};
use dioxus_i18n::{prelude::*, t};
use std::path::PathBuf;
use unic_langid::langid;
use views::{
    About, AdminAnalytics, AdminContent, AdminCreateBlogPost, AdminCreateDiscount,
    AdminCreateProduct, AdminCreateStockItem, AdminStockLocations, AdminUsers, AdminDiscounts, AdminEditBlogPost,
    AdminEditDiscount, AdminEditProduct, AdminEditStockItem, AdminInventory, AdminOrders,
    AdminProducts, AdminSettings, AdminCreateGroup, AdminEditGroup, AdminGroups, BlogPostPage, BlogPosts, Cart, Checkout, CheckoutPayment,
    Collection, Contact, Dashboard as AdminDashboard, UserDashboard, Faq, Home, NotFound, GroupPage,
    OrderStatus, PeptideCalculator, Policies, ProductPage, ShippingPolicy, SignIn as AdminSignIn,
    VerifyMagicLink,
};
mod backend;
mod components;
mod i18n;
mod utils;
mod views;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    // PUBLIC ROUTES
    #[layout(HeaderFooter)]
        #[route("/")]
        Home {},
        #[route("/dashboard")]
        UserDashboard {},
        #[nest("/categories")]
            #[route("/:codename")]
            Collection { codename: String },
        #[end_nest]
        #[nest("/products")]
            #[route("/:handle")]
            ProductPage { handle: String },
        #[end_nest]
        #[route("/contact")]
        Contact {},
        #[route("/about")]
        About {},
        #[route("/faq")]
        Faq {},
        #[nest("/policies")]
            #[route("")]
            Policies {},
            #[route("/shipping")]
            ShippingPolicy {},
        #[end_nest]
        #[route("/blog/:id")]
        BlogPostPage { id: String },
        #[route("/blog")]
        BlogPosts {},
        #[route("/groups/:id")]
        GroupPage { id: String },
        #[route("/cart")]
        Cart {},
        #[route("/order/:order_id")]
        OrderStatus { order_id: String },
    #[end_layout]
    #[layout(CheckoutHeader)]
        #[nest("/checkout")]
            #[route("")]
            Checkout {},
            #[route("/pay/:payment_id")]
            CheckoutPayment { payment_id: String},
        #[end_nest]
    #[end_layout]
    // PRIVATE ROUTES
    #[nest("/admin")]
        #[layout(AdminWrapper)]
            #[route("/dashboard")]
            AdminDashboard {},
            #[route("/dashboard/orders")]
            AdminOrders {},
            #[nest("/dashboard/products")]
                #[route("")]
                AdminProducts {},
                #[route("/create")]
                AdminCreateProduct {},
                #[route("/edit/:id")]
                AdminEditProduct { id: String },
            #[end_nest]
            #[nest("/dashboard/inventory")]
                #[route("")]
                AdminInventory {},
                #[route("/create")]
                AdminCreateStockItem {},
                #[route("/edit/:id")]
                AdminEditStockItem { id: String },
                #[route("/locations")]
                AdminStockLocations {},
            #[end_nest]
            #[route("/dashboard/users")]
            AdminUsers {},
            #[nest("/dashboard/discounts")]
                #[route("")]
                AdminDiscounts {},
                #[route("/create")]
                AdminCreateDiscount {},
                #[route("/edit/:id")]
                AdminEditDiscount { id: String },
            #[end_nest]
            #[nest("/dashboard/content")]
                #[route("")]
                AdminContent {},
                #[route("/create")]
                AdminCreateBlogPost {},
                #[route("/edit/:id")]
                AdminEditBlogPost { id: String },
            #[end_nest]
            #[nest("/dashboard/groups")]
                #[route("")]
                AdminGroups {},
                #[route("/create")]
                AdminCreateGroup {},
                #[route("/edit/:id")]
                AdminEditGroup { id: String },
            #[end_nest]
            #[route("/dashboard/analytics")]
            AdminAnalytics {},
            #[route("/dashboard/settings")]
            AdminSettings {},
        #[end_layout]
        #[route("/signin")]
        AdminSignIn {},
        #[route("/verify")]
        VerifyMagicLink {},
    #[end_nest]
    // REDIRECTS
    #[nest("/categories")]
        #[redirect("/", || Route::Collection { codename: String::from("all") })]
        #[redirect("/:codename", |codename: String| Route::Collection { codename })]
    #[end_nest]
    #[nest("/category")]
        #[redirect("/", || Route::Home {})]
        #[redirect("/:codename", |codename: String| Route::Collection { codename })]
    #[end_nest]
    #[redirect("/product/:handle", |handle: String| Route::ProductPage { handle })]
    #[redirect("/a/faq", || Route::Faq {})]
    #[redirect("/about-us", || Route::About {})]
    // 404
    #[layout(HeaderFooter)]
    #[route("/:..route")]
    NotFound { route: Vec<String> },
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/styling/main.css");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

// Sitemap handler
#[cfg(feature = "server")]
async fn sitemap_handler() -> Response {
    match generate_sitemap().await {
        Ok(xml) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/xml")],
            xml,
        )
            .into_response(),
        Err(e) => {
            eprintln!("Error generating sitemap: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Error generating sitemap",
            )
                .into_response()
        }
    }
}

// Robots.txt handler
#[cfg(feature = "server")]
async fn robots_handler() -> Response {
    let robots_txt = format!(
        "User-agent: *\nAllow: /\nSitemap: https://noveldist.com/sitemap.xml"
    );

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain")],
        robots_txt,
    )
        .into_response()
}

// The entry point for the server
#[cfg(feature = "server")]
#[tokio::main]
async fn main() {
    dotenv().ok();
    let app_state = setup_app_state().await.expect("Failed to set up app state");
    let address = dioxus::cli_config::fullstack_address_or_localhost();

    // Spawn Meta cache for SEO
    println!("Starting meta cache refresh task...");
    meta_cache::start_meta_cache_refresh_task();

    // Spawn background task for checking payments every 5 minutes
    tokio::spawn(async {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300)); // 5 minutes

        loop {
            interval.tick().await;

            println!("Running payment status check...");
            match payments::check_payments().await {
                Ok(()) => {
                    println!("Payment status check completed successfully");
                }
                Err(e) => {
                    println!("Error during payment status check: {:?}", e);
                }
            }
        }
    });

    fn get_upload_path() -> &'static str {
        if env::var("RAILWAY_ENVIRONMENT").is_ok() {
            "/app/assets/uploads" // Railway volume mount path
        } else {
            "assets/uploads" // Local development path
        }
    }

    let upload_path = get_upload_path();
    let router = axum::Router::new()
        .serve_dioxus_application(dioxus::server::ServeConfig::default(), App)
        // Add sitemap endpoint
        .route("/sitemap.xml", axum::routing::get(sitemap_handler))
        // Add robots.txt endpoint
        .route("/robots.txt", axum::routing::get(robots_handler))
        // Invite code API
        .route("/novapi/v1/groups/invite/generate", axum::routing::post(generate_invite_code_handler))
        .nest_service("/public", ServeDir::new("assets/images/public"))
        .layer(axum::middleware::from_fn(inject_meta_middleware))
        .nest_service("/uploads", ServeDir::new(upload_path)) // Add uploads route
        .layer(axum::middleware::from_fn_with_state(
            app_state,
            auth_middleware,
        ));

    let router = router.into_make_service();
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();
    axum::serve(listener, router).await.unwrap();
}

#[cfg(not(feature = "server"))]
fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    // Initialize i18n with default language
    let _ = use_init_i18n(|| config(langid!("en-US")));
    // Initialize user preferred language hook
    let _language = use_user_preferred_language();

    // Initialize SmilesDrawer library once for the entire app
    use_effect(move || {
        const SMILES_ASSET: Asset = asset!(
            "/assets/smiles-drawer-2.min.js",
            JsAssetOptions::new().with_minify(false)
        );

        let script_code = format!(
            r#"
            if (!window.smilesDrawerLoaded) {{
                const script = document.createElement('script');
                script.src = '{}';
                script.onload = function() {{
                    console.log('SmilesDrawer 2.1.7 loaded globally');

                    // Initialize a global drawer manager that creates drawers per size
                    window.smilesDrawerInstances = {{}};

                    window.getSmilesDrawer = function(width, height, padding) {{
                        const key = width + '_' + height + '_' + padding;
                        if (!window.smilesDrawerInstances[key]) {{
                            console.log('Creating new SvgDrawer instance for size:', key);
                            window.smilesDrawerInstances[key] = new SmilesDrawer.SvgDrawer({{
                                width: width,
                                height: height,
                                padding: padding,
                                compactDrawing: false,
                                bondThickness: 1.7
                            }});
                        }}
                        return window.smilesDrawerInstances[key];
                    }};

                    window.smilesDrawerLoaded = true;
                }};
                document.head.appendChild(script);
            }}
            "#,
            SMILES_ASSET
        );
        document::eval(&script_code);
    });

    // Initialize Chart.js library once for the entire app
    use_effect(move || {
        const CHARTJS_ASSET: Asset = asset!(
            "/assets/chart.min.js",
            JsAssetOptions::new().with_minify(false)
        );

        let script_code = format!(
            r#"
            if (!window.chartJsLoaded) {{
                const script = document.createElement('script');
                script.src = '{}';
                script.onload = function() {{
                    console.log('Chart.js loaded globally');
                    window.chartJsLoaded = true;
                }};
                document.head.appendChild(script);
            }}
            "#,
            CHARTJS_ASSET
        );
        document::eval(&script_code);
    });

    rsx! {
        // Head links
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        // Add analytics
        //REMOVE THIS
        /*
        document::Script {
            defer: true,
            src: "https://stats.penchant.bio/script.js",
            "data-website-id": "636b82ba-3416-4f17-b1da-b78cf99738ce"
        }

        */
        // Base meta tags (these will be on every page)
        document::Meta {
            charset: "utf-8"
        }
        document::Meta {
            name: "viewport",
            content: "width=device-width, initial-scale=1.0"
        }
        document::Meta {
            property: "og:site_name",
            content: "Novel Distributions"
        }
        document::Meta {
            property: "og:type",
            content: "website"
        }
        document::Meta {
            name: "twitter:site",
            content: "@PenchantBio"
        }
        Router::<Route> {}
    }
}
