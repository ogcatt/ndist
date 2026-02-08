mod admin_dashboard;
pub use admin_dashboard::Dashboard;

mod signin;
pub use signin::SignIn;

mod verify;
pub use verify::VerifyMagicLink;

mod admin_orders;
pub use admin_orders::AdminOrders;

mod admin_products;
pub use admin_products::AdminProducts;

mod admin_inventory;
pub use admin_inventory::AdminInventory;

mod admin_customers;
pub use admin_customers::AdminCustomers;

mod admin_discounts;
pub use admin_discounts::AdminDiscounts;

mod admin_content;
pub use admin_content::AdminContent;

mod admin_analytics;
pub use admin_analytics::AdminAnalytics;

mod admin_settings;
pub use admin_settings::AdminSettings;

// SUB-PAGES
// SUB-PAGES:

mod sub_pages;
pub use sub_pages::AdminCreateBlogPost;
pub use sub_pages::AdminCreateDiscount;
pub use sub_pages::AdminCreateProduct;
pub use sub_pages::AdminCreateStockItem;
pub use sub_pages::AdminEditDiscount;
pub use sub_pages::AdminEditProduct;
pub use sub_pages::AdminEditStockItem;
pub use sub_pages::AdminEditBlogPost;
pub use sub_pages::AdminProduct;
