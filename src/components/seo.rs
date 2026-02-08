use dioxus::prelude::*;

#[derive(Props, PartialEq, Clone)]
pub struct MetaPageProps {
    #[props(default = "Novel Distributions | Delivering innovation worldwide.".to_string())]
    pub title: String,
    #[props(default = "Storefront for innovative biomodulators.".to_string())]
    pub description: String,
    #[props(default = "https://noveldist.com".to_string())]
    pub domain: String,
    #[props(default = "".to_string())]
    pub image_url: String,
    #[props(default = 1500)]
    pub w: u32,
}

#[component]
pub fn Meta(props: MetaPageProps) -> Element {
    let router = use_router();

    // Get the current route
    let current_route = router.full_route_string();

    // Format title
    let formatted_title = if !props.title.starts_with("Novel Distributions") && !props.title.ends_with("Novel Distributions") {
        format!("{} | Novel Distributions", props.title)
    } else {
        props.title.clone()
    };

    // Process image URL
    let actual_image_url = if !props.image_url.is_empty() {
        if props.image_url.starts_with("http") {
            props.image_url.clone()
        } else {
            format!("{}{}", props.domain, props.image_url)
        }
    } else {
        format!("{}/assets/images/preview.jpg", props.domain)
    };

    // Build canonical URL
    let canonical_url = format!("{}{}", props.domain, current_route);

    rsx! {
        document::Title { "{formatted_title}" }

        // Basic meta tags
        document::Meta {
            name: "description",
            content: "{props.description}"
        }

        // Open Graph tags
        document::Meta {
            property: "og:type",
            content: "website"
        }
        document::Meta {
            property: "og:title",
            content: "{formatted_title}"
        }
        document::Meta {
            property: "og:description",
            content: "{props.description}"
        }
        document::Meta {
            property: "og:url",
            content: "{canonical_url}"
        }
        document::Meta {
            property: "og:image",
            content: "{actual_image_url}"
        }
        document::Meta {
            property: "og:image:secure_url",
            content: "{actual_image_url}"
        }
        document::Meta {
            property: "og:image:width",
            content: "{props.w}"
        }
        document::Meta {
            property: "og:image:height",
            content: "1000"
        }

        // Twitter Card tags
        document::Meta {
            name: "twitter:card",
            content: "summary_large_image"
        }
        document::Meta {
            name: "twitter:title",
            content: "{formatted_title}"
        }
        document::Meta {
            name: "twitter:description",
            content: "{props.description}"
        }
        document::Meta {
            name: "twitter:image",
            content: "{actual_image_url}"
        }

        // Canonical link
        document::Link {
            rel: "canonical",
            href: "{canonical_url}"
        }
    }
}
