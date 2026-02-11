use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::backend::server_functions;

#[derive(Debug, Clone)]
pub struct MetaTags {
    pub title: String,
    pub description: String,
    pub image_url: Option<String>,
    pub url: String,
    pub og_type: String,
    pub json_ld: Option<String>,
}

impl MetaTags {
    pub fn new(title: String, description: String, url: String) -> Self {
        Self {
            title,
            description,
            image_url: None,
            url,
            og_type: "website".to_string(),
            json_ld: None,
        }
    }

    pub fn with_image(mut self, image_url: String) -> Self {
        self.image_url = Some(image_url);
        self
    }

    pub fn with_json_ld(mut self, json_ld: String) -> Self {
        self.json_ld = Some(json_ld);
        self
    }

    pub fn to_html(&self) -> String {
        let image_url = self.image_url
            .as_ref()
            .map(|url| {
                if url.starts_with("http") {
                    url.clone()
                } else {
                    format!("https://noveldist.com{}", url)
                }
            })
            .unwrap_or_else(|| "https://noveldist.com/assets/images/preview.jpg".to_string());

        let mut html = format!(
            r#"
<title>{title}</title>
<meta name="description" content="{description}" />
<meta property="og:type" content="{og_type}" />
<meta property="og:title" content="{title}" />
<meta property="og:description" content="{description}" />
<meta property="og:url" content="{url}" />
<meta property="og:image" content="{image}" />
<meta property="og:image:secure_url" content="{image}" />
<meta property="og:image:width" content="1500" />
<meta property="og:image:height" content="1000" />
<meta name="twitter:card" content="summary_large_image" />
<meta name="twitter:title" content="{title}" />
<meta name="twitter:description" content="{description}" />
<meta name="twitter:image" content="{image}" />
<link rel="canonical" href="{url}" />"#,
            title = html_escape(&self.title),
            description = html_escape(&self.description),
            og_type = &self.og_type,
            url = &self.url,
            image = &image_url
        );

        // Add JSON-LD if available
        if let Some(ref json_ld) = self.json_ld {
            html.push_str(&format!(
                r#"
<script type="application/ld+json">
{}
</script>"#,
                json_ld
            ));
        }

        html
    }
}

// Simple HTML escaping for meta tag content
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

// Global cache for meta tags
static META_CACHE: Lazy<Arc<RwLock<HashMap<String, MetaTags>>>> = Lazy::new(|| {
    Arc::new(RwLock::new(HashMap::new()))
});

/// Get meta tags from cache for a given path
pub async fn get_meta_tags(path: &str) -> Option<MetaTags> {
    let cache = META_CACHE.read().await;
    cache.get(path).cloned()
}

/// Get meta tags or return default
pub async fn get_meta_tags_or_default(path: &str) -> MetaTags {
    get_meta_tags(path).await.unwrap_or_else(|| {
        MetaTags::new(
            "Novel Distributions | Delivering innovation worldwide".to_string(),
            "Storefront for innovative biomodulators.".to_string(),
            format!("https://noveldist.com{}", path)
        )
    })
}

/// Generate JSON-LD structured data for a product
fn generate_product_json_ld(product: &crate::backend::front_entities::Product) -> String {
    // Get the first variant for price information
    let (price, currency, availability) = if let Some(ref variants) = product.variants {
        if let Some(variant) = variants.first() {
            let avail = if product.force_no_stock {
                "OutOfStock"
            } else if let Some(qty) = variant.calculated_stock_quantity {
                if qty > 0 { "InStock" } else if product.back_order { "PreOrder" } else { "OutOfStock" }
            } else {
                "OutOfStock"
            };
            (variant.price_standard_usd, "USD", avail)
        } else {
            (0.0, "USD", "OutOfStock")
        }
    } else {
        (0.0, "USD", "OutOfStock")
    };

    // Get product image
    let image_url = if let Some(ref variants) = product.variants {
        if let Some(variant) = variants.first() {
            if let Some(ref thumbnail_url) = variant.thumbnail_url {
                if thumbnail_url.starts_with("http") {
                    thumbnail_url.clone()
                } else {
                    format!("https://noveldist.com{}", thumbnail_url)
                }
            } else {
                "https://noveldist.com/assets/images/preview.jpg".to_string()
            }
        } else {
            "https://noveldist.com/assets/images/preview.jpg".to_string()
        }
    } else {
        "https://noveldist.com/assets/images/preview.jpg".to_string()
    };

    // Build the JSON-LD
    let mut json_ld = serde_json::json!({
        "@context": "https://schema.org",
        "@type": "Product",
        "name": product.title,
        "description": product.small_description_md.as_ref().map(|s| strip_html_tags(s)).unwrap_or_else(|| product.title.clone()),
        "url": format!("https://noveldist.com/products/{}", product.handle),
        "image": image_url,
        "offers": {
            "@type": "Offer",
            "price": price,
            "priceCurrency": currency,
            "availability": format!("https://schema.org/{}", availability),
            "url": format!("https://noveldist.com/products/{}", product.handle),
            "seller": {
                "@type": "Organization",
                "name": "Novel Distributions"
            }
        },
        "brand": {
            "@type": "Brand",
            "name": product.brand.as_ref().unwrap_or(&"Novel Distributions".to_string())
        }
    });

    // Add SKU if available
    if let Some(ref variants) = product.variants {
        if let Some(variant) = variants.first() {
            if let Some(ref sku) = variant.pbx_sku {
                json_ld["sku"] = serde_json::json!(sku);
            }
        }
    }

    // Add additional properties if available
    if let Some(ref cas) = product.cas {
        json_ld["additionalProperty"] = serde_json::json!([
            {
                "@type": "PropertyValue",
                "name": "CAS Number",
                "value": cas
            }
        ]);
    }

    // Add purity as a property
    if let Some(purity) = product.purity {
        let mut props = json_ld["additionalProperty"].as_array().cloned().unwrap_or_default();
        props.push(serde_json::json!({
            "@type": "PropertyValue",
            "name": "Purity",
            "value": format!("{}%", purity)
        }));
        json_ld["additionalProperty"] = serde_json::json!(props);
    }

    // Add molecular formula
    if let Some(ref mol_form) = product.mol_form {
        let mut props = json_ld["additionalProperty"].as_array().cloned().unwrap_or_default();
        props.push(serde_json::json!({
            "@type": "PropertyValue",
            "name": "Molecular Formula",
            "value": mol_form
        }));
        json_ld["additionalProperty"] = serde_json::json!(props);
    }

    serde_json::to_string_pretty(&json_ld).unwrap_or_default()
}

/// Strip HTML tags from a string (simple implementation)
fn strip_html_tags(html: &str) -> String {
    let re = regex::Regex::new(r"<[^>]*>").unwrap();
    re.replace_all(html, "").trim().to_string()
}

/// Refresh the meta cache with current data from the database
pub async fn refresh_meta_cache() -> Result<(), Box<dyn std::error::Error>> {
    let mut new_cache = HashMap::new();

    // Static routes
    new_cache.insert(
        "/".to_string(),
        MetaTags::new(
            "Novel Distributions | Delivering innovation worldwide".to_string(),
            "Storefront for innovative biomodulators.".to_string(),
            "https://noveldist.com/".to_string()
        )
    );

    new_cache.insert(
        "/categories".to_string(),
        MetaTags::new(
            "Categories | Novel Distributions".to_string(),
            "Browse our categories.".to_string(),
            "https://noveldist.com/categories".to_string()
        )
    );

    new_cache.insert(
        "/about".to_string(),
        MetaTags::new(
            "About | Novel Distributions".to_string(),
            "Learn about Novel Distributions.".to_string(),
            "https://noveldist.com/about".to_string()
        )
    );

    new_cache.insert(
        "/contact".to_string(),
        MetaTags::new(
            "Contact | Novel Distributions".to_string(),
            "Get in touch with Novel Distributions for general inquiries.".to_string(),
            "https://noveldist.com/contact".to_string()
        )
    );

    new_cache.insert(
        "/faq".to_string(),
        MetaTags::new(
            "FAQ | Novel Distributions".to_string(),
            "Frequently asked questions about Novel Distributions products and services.".to_string(),
            "https://noveldist.com/faq".to_string()
        )
    );

    new_cache.insert(
        "/policies".to_string(),
        MetaTags::new(
            "Terms of Service | Novel Distributions".to_string(),
            "Read the Novel Distributions Terms of Service.".to_string(),
            "https://noveldist.com/policies".to_string()
        )
    );

    new_cache.insert(
        "/policies/shipping".to_string(),
        MetaTags::new(
            "Shipping Policy | Novel Distributions".to_string(),
            "Learn about our shipping policies.".to_string(),
            "https://noveldist.com/policies/shipping".to_string()
        )
    );

    // REMOVE THIS
    new_cache.insert(
        "/blog".to_string(),
        MetaTags::new(
            "Blog | Novel Distributions".to_string(),
            ".".to_string(),
            "https://noveldist.com/blog".to_string()
        )
    );

    // REMOVE THIS
    new_cache.insert(
        "/peptide-calculator".to_string(),
        MetaTags::new(
            "Peptide Calculator | Novel Distributions".to_string(),
            "Calculate peptide reconstitution and dosing with our free online tool.".to_string(),
            "https://noveldist.com/peptide-calculator".to_string()
        )
    );

    new_cache.insert(
        "/cart".to_string(),
        MetaTags::new(
            "Cart | Novel Distributions".to_string(),
            "Review your cart and proceed to checkout.".to_string(),
            "https://noveldist.com/cart".to_string()
        )
    );

    // Dynamic routes - Products
    match server_functions::get_products().await {
        Ok(products) => {
            for product in &products {
                // Get product form as plain English string (avoiding i18n context)
                let product_form_str = match product.product_form {
                    crate::backend::front_entities::ProductForm::Ampoule => "Ampoule",
                    crate::backend::front_entities::ProductForm::Capsules => "Capsules",
                    crate::backend::front_entities::ProductForm::Container => "Container",
                    crate::backend::front_entities::ProductForm::DirectSpray => "Spray",
                    crate::backend::front_entities::ProductForm::Multi => "Multi",
                    crate::backend::front_entities::ProductForm::Other => "Other",
                    crate::backend::front_entities::ProductForm::Solution => "Solution",
                    crate::backend::front_entities::ProductForm::VerticalSpray => "Spray",
                    crate::backend::front_entities::ProductForm::Vial => "Vial",
                };

                let title = if product.title.contains("(") {
                    format!("Buy {} | Novel Distributions", product.title)
                } else {
                    format!("Buy {} ({}) | Novel Distributions", product.title, product_form_str)
                };

                let mut description = format!(
                    "Purchase {}",
                    product.title
                );

                // Add alternate names
                if let Some(ref alternate_names) = product.alternate_names {
                    if !alternate_names.is_empty() {
                        description.push_str(&format!(
                            " ({})",
                            alternate_names.join("/")
                        ));
                    }
                }

                description.push_str(" at Novel Distributions - Delivering Worldwide");

                // Add purity
                if let Some(ref purity) = product.purity {
                    description.push_str(&format!(", {}% pure", purity));
                }

                // Add CAS
                if let Some(ref cas) = product.cas {
                    description.push_str(&format!(", CAS: {}", cas));
                }

                let url = format!("https://noveldist.com/products/{}", product.handle);

                let mut meta = MetaTags::new(title, description, url);

                // Add product image if available
                if let Some(ref variants) = product.variants {
                    if let Some(variant) = variants.first() {
                        if let Some(ref thumbnail_url) = variant.thumbnail_url {
                            if !thumbnail_url.is_empty() && thumbnail_url != "smiles" {
                                meta = meta.with_image(thumbnail_url.clone());
                            }
                        }
                    }
                }

                // Add JSON-LD structured data for products
                let json_ld = generate_product_json_ld(&product);
                meta = meta.with_json_ld(json_ld);

                new_cache.insert(
                    format!("/products/{}", product.handle),
                    meta
                );
            }
            println!("✓ Cached meta tags for {} products", products.len());
        }
        Err(e) => {
            eprintln!("Error fetching products for meta cache: {:?}", e);
        }
    }

    // Dynamic routes - Blog Posts
    match server_functions::get_blog_posts().await {
        Ok(posts) => {
            for post in &posts {
                let title = format!("{} | Novel Distributions", post.title);

                // Use subtitle if available, otherwise create a generic description
                let description = if let Some(ref subtitle) = post.subtitle {
                    subtitle.clone()
                } else {
                    "Read more on the Novel Distributions blog.".to_string()
                };

                let url = format!("https://noveldist.com/blog/{}", post.id);

                let mut meta = MetaTags::new(title, description, url);

                // Add blog post image if available (using thumbnail_url field)
                if let Some(ref image_url) = post.thumbnail_url {
                    if !image_url.is_empty() {
                        meta = meta.with_image(image_url.clone());
                    }
                }

                new_cache.insert(
                    format!("/blog/{}", post.id),
                    meta
                );
            }
            println!("✓ Cached meta tags for {} blog posts", posts.len());
        }
        Err(e) => {
            eprintln!("Error fetching blog posts for meta cache: {:?}", e);
        }
    }

    // Update the global cache
    let mut cache = META_CACHE.write().await;
    *cache = new_cache;

    println!("✓ Meta cache refreshed with {} entries", cache.len());

    Ok(())
}

/// Start background task to refresh meta cache periodically
pub fn start_meta_cache_refresh_task() {
    tokio::spawn(async {
        // Initial refresh
        if let Err(e) = refresh_meta_cache().await {
            eprintln!("Error during initial meta cache refresh: {:?}", e);
        }

        // Refresh every 60 seconds
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));

        loop {
            interval.tick().await;

            if let Err(e) = refresh_meta_cache().await {
                eprintln!("Error during meta cache refresh: {:?}", e);
            }
        }
    });
}
