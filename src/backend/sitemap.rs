use chrono::{DateTime, Utc};
use crate::backend::server_functions;
use crate::backend::front_entities::Category;
use strum::IntoEnumIterator;

#[derive(Debug, Clone)]
pub struct SitemapUrl {
    pub loc: String,
    pub lastmod: Option<String>,
    pub changefreq: Option<String>,
    pub priority: Option<f32>,
}

impl SitemapUrl {
    pub fn new(loc: String) -> Self {
        Self {
            loc,
            lastmod: Some(Utc::now().format("%Y-%m-%d").to_string()),
            changefreq: None,
            priority: None,
        }
    }

    pub fn with_priority(mut self, priority: f32) -> Self {
        self.priority = Some(priority);
        self
    }

    pub fn with_changefreq(mut self, changefreq: &str) -> Self {
        self.changefreq = Some(changefreq.to_string());
        self
    }

    pub fn with_lastmod(mut self, lastmod: String) -> Self {
        self.lastmod = Some(lastmod);
        self
    }
}

pub async fn generate_sitemap() -> Result<String, Box<dyn std::error::Error>> {
    let base_url = "https://noveldist.com";
    let mut urls = Vec::new();

    // Static routes
    urls.push(
        SitemapUrl::new(format!("{}/", base_url))
            .with_priority(1.0)
            .with_changefreq("daily")
    );

    urls.push(
        SitemapUrl::new(format!("{}/categories", base_url))
            .with_priority(0.9)
            .with_changefreq("daily")
    );

    urls.push(
        SitemapUrl::new(format!("{}/about", base_url))
            .with_priority(0.8)
            .with_changefreq("monthly")
    );

    urls.push(
        SitemapUrl::new(format!("{}/contact", base_url))
            .with_priority(0.7)
            .with_changefreq("monthly")
    );

    urls.push(
        SitemapUrl::new(format!("{}/faq", base_url))
            .with_priority(0.7)
            .with_changefreq("monthly")
    );

    urls.push(
        SitemapUrl::new(format!("{}/policies", base_url))
            .with_priority(0.6)
            .with_changefreq("monthly")
    );

    urls.push(
        SitemapUrl::new(format!("{}/policies/shipping", base_url))
            .with_priority(0.6)
            .with_changefreq("monthly")
    );

    urls.push(
        SitemapUrl::new(format!("{}/blog", base_url))
            .with_priority(0.8)
            .with_changefreq("weekly")
    );

    // Dynamic routes - Products
    match server_functions::get_products().await {
        Ok(products) => {
            for product in products {
                urls.push(
                    SitemapUrl::new(format!("{}/products/{}", base_url, product.handle))
                        .with_priority(0.9)
                        .with_changefreq("weekly")
                        .with_lastmod(product.updated_at.format("%Y-%m-%d").to_string())
                );
            }
        }
        Err(e) => {
            eprintln!("Error fetching products for sitemap: {:?}", e);
        }
    }

    // Static collection routes based on Category enum
    for category in Category::iter() {
        let key = category.to_key();
        urls.push(
            SitemapUrl::new(format!("{}/collections/{}", base_url, key))
                .with_priority(0.85)
                .with_changefreq("weekly")
        );
    }

    // Dynamic routes - Blog Posts
    match server_functions::get_blog_posts().await {
        Ok(posts) => {
            for post in posts {
                urls.push(
                    SitemapUrl::new(format!("{}/blog/{}", base_url, post.id))
                        .with_priority(0.7)
                        .with_changefreq("monthly")
                        .with_lastmod(post.updated_at.format("%Y-%m-%d").to_string())
                );
            }
        }
        Err(e) => {
            eprintln!("Error fetching blog posts for sitemap: {:?}", e);
        }
    }

    // Generate XML
    let mut xml = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    xml.push_str("\n");
    xml.push_str(r#"<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">"#);
    xml.push_str("\n");

    for url in urls {
        xml.push_str("  <url>\n");
        xml.push_str(&format!("    <loc>{}</loc>\n", url.loc));

        if let Some(lastmod) = url.lastmod {
            xml.push_str(&format!("    <lastmod>{}</lastmod>\n", lastmod));
        }

        if let Some(changefreq) = url.changefreq {
            xml.push_str(&format!("    <changefreq>{}</changefreq>\n", changefreq));
        }

        if let Some(priority) = url.priority {
            xml.push_str(&format!("    <priority>{:.1}</priority>\n", priority));
        }

        xml.push_str("  </url>\n");
    }

    xml.push_str("</urlset>");

    Ok(xml)
}
