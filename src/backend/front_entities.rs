use chrono::NaiveDateTime;
use dioxus_i18n::t;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use strum_macros::{Display, EnumIter, EnumString};

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Product {
    // Unique ID of the product
    pub id: String,
    // Main product name
    pub title: String,
    // Product subtitle (shown under title), e.g 15mg/mL
    pub subtitle: Option<String>,
    // Other product names (used for metadata)
    pub alternate_names: Option<Vec<String>>,
    // Product handle used for the URL and "nicer" technical references
    pub handle: String,
    // Collections this product is in
    pub collections: Option<Vec<String>>,
    // Form of product; Ampoule, Capsules, Container, DirectSpray, Multi, Other, Solution, VerticalSpray, Vial
    pub product_form: ProductForm, // Updated to use enum
    // Description of the product in physical terms (packaging etc.)
    pub physical_description: Option<String>,
    // The default variant which is displayed
    pub default_variant_id: Option<String>,
    // Forces a product which may have inventory to be out of stock
    pub force_no_stock: bool,
    // Penchant Labs Node ID reference for this specific product
    pub plabs_node_id: Option<String>,
    // Standard purity of the specific product
    pub purity: Option<f64>,
    // Product visibility; Private, Public or Unlisted
    pub visibility: ProductVisibility, // Updated to use enum
    // The summary-like description that is shown at the top of a product page / When created, it is converted into html to render
    pub small_description_md: Option<String>,
    // The main description shown at the bottom of a product page / When created, it is converted into html to render
    pub main_description_md: Option<String>,
    // CAS code
    pub cas: Option<String>,
    // IUPAC code
    pub iupac: Option<String>,
    // Molecular Formula
    pub mol_form: Option<String>,
    // Smiles Code
    pub smiles: Option<String>,
    // Allows the smiles to be rendered (may be disabled if smiles is too long)
    pub enable_render_if_smiles: bool,
    // Pubchem CID of the compound (used to open Pubchem page)
    pub pubchem_cid: Option<String>,
    // Experimental ADMET profile calculation (using ADMETlab 3.0)
    pub calculated_admet: Option<f64>,
    // URL for QNMR purity analysis folder
    pub analysis_url_qnmr: Option<String>,
    // URL for HPLC purity analysis folder
    pub analysis_url_hplc: Option<String>,
    // URL for Q-H1 purity analysis folder
    pub analysis_url_qh1: Option<String>,
    // Weight of a single product in grams (applies to all variants unless they have their own weight)
    pub weight: Option<f64>,
    // Height of product in MM
    pub dimensions_height: Option<f64>,
    // Length of product in MM
    pub dimensions_length: Option<f64>,
    // Width of product in MM
    pub dimensions_width: Option<f64>,
    // When the product was created
    pub created_at: NaiveDateTime,
    // When the product was last updated
    pub updated_at: NaiveDateTime,
    // The product phase (major category) of this product
    pub phase: ProductPhase, // Updated to use enum
    // If true, the item is marked as a pre-order (will be seperate to main products & will be used to mark an order)
    pub pre_order: bool,
    // The goal is how many sales of this product is needed for completion.
    pub pre_order_goal: Option<f64>,
    // The priority of this product compared to others (used to order features)
    pub priority: Option<i32>,
    // The special brand name of this product (if not of the store)
    pub brand: Option<String>,
    // If this product can be backordered (default false)
    pub back_order: bool,
    // The main mechanism of this product/molecule (e.g NMDA PAM)
    pub mechanism: Option<String>,
    // Extra meatadata information (can be used to pass extra information without db change)
    pub metadata: Option<String>,
    // If not null only these groups can access and purchase this product
    pub access_groups: Option<Vec<String>>,
    // If not null only these specific user IDs can access and purchase this product (in addition to any access_groups)
    pub access_users: Option<Vec<String>>,
    // If this is true then even if the product is gated to specific groups it shows a bare preview on home/product browser
    pub show_private_preview: bool,

    // SUB-DATA
    pub variants: Option<Vec<ProductVariants>>, // Updated to use struct
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct ProductVariants {
    // Unique ID of the Product Variant
    pub id: String,
    // Name of the variant (e.g 15g or 15mL)
    pub variant_name: String,
    // ID of the associated parent Product
    pub product_id: String,
    // SKU for this variant (prefixed with PBX)
    pub pbx_sku: Option<String>,
    // Public main thumbnail URL for associated Variant
    pub thumbnail_url: Option<String>,
    // Weight of a single variant in grams (applies to this variant and ignores father product weight)
    pub weight: Option<f64>,
    // Standard price of Variant (this can be sale price)
    pub price_standard_usd: f64,
    // Standard price of Variant without deductions (such as sale)
    pub price_standard_without_sale: Option<f64>,
    // Array of extra thumbnails for variant
    pub additional_thumbnail_urls: Option<Vec<String>>,
    // Calculated stock quantity value for the client to use (calculated from linked stock items)
    pub calculated_stock_quantity: Option<i32>,
    // When the Variant was created
    pub created_at: NaiveDateTime,
    // When the variant was last updated
    pub updated_at: NaiveDateTime,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct CustomerBasketItem {
    // Unique basket item ID
    pub id: String,
    // Basket ID this item is part of
    pub basket_id: String,
    // Variant ID this item is (is automatically related to product)
    pub product_variant_id: String,
    // Quantity of this basket item
    pub quantity: i32,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct BasketDiscountData {
    // The type of discount added to the basket
    pub discount_type: DiscountType,
    // If a percentage type, the percentage off
    pub discount_percentage: Option<f64>,
    // If an amount type, the $ amount that should be discounted
    pub discount_amount_left: Option<f64>,
    // If a discount is of type auto
    pub discount_auto_apply: bool,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct CustomerBasket {
    // Unique ID of this basket
    pub id: String,
    // The ID of the customer if they are logged in (used to persist basket)
    pub customer_id: Option<String>,
    // The country of this order (ISO 3166), used for shipping calculations
    pub country_code: Option<String>,
    // The discount code of this order (not ID), should be checked for validity every cart request
    pub discount_code: Option<String>,
    // The shipping option selected for this order (non-db calc, only able to be set if country matches)
    pub shipping_option: Option<ShippingOption>,
    // The selected stock location for this basket session
    pub stock_location_id: Option<String>,
    // Calculated shipping option results (temporary for frontend, recalculated per-request)
    pub shipping_results: Option<Vec<ShippingResult>>,
    // If this cart is locked and can't be modified (used when payment is active)
    pub locked: bool,
    // If a payment ID is active then this is the ID of the associated payment entry
    pub payment_id: Option<String>,
    // When the payment failed (if expired) - this should show for 8h after payment failure
    pub payment_failed_at: Option<NaiveDateTime>,
    // When the basket was created
    pub created_at: NaiveDateTime,
    // When the basket was last updated
    pub updated_at: NaiveDateTime,
    // Items on this basket
    pub items: Option<Vec<CustomerBasketItem>>,
    // If a discount is applied to this basket, this contains the details about it
    pub discount: Option<BasketDiscountData>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CheckCartResult {
    Complete,
    Reduced,
    Removed,
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, Serialize, Deserialize, Hash)]
pub enum ShippingOption {
    Tracked,
    Express,
    TrackedUS,
}

impl ShippingOption {
    pub fn to_string(&self) -> String {
        match self {
            ShippingOption::Tracked => t!("shipping-option-tracked"),
            ShippingOption::Express => t!("shipping-option-express"),
            ShippingOption::TrackedUS => t!("shipping-option-tracked-us"),
        }
    }

    pub fn to_description(&self) -> String {
        match self {
            ShippingOption::Tracked => t!("shipping-option-tracked-description"),
            ShippingOption::Express => t!("shipping-option-express-description"),
            ShippingOption::TrackedUS => t!("shipping-option-tracked-us-description"),
        }
    }
}

impl fmt::Display for ShippingOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct ShippingResult {
    pub option: ShippingOption,
    pub cost_usd: f64,
    pub estimated_days: String,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct ShippingQuote {
    pub available_options: Vec<ShippingResult>,
}

impl CheckCartResult {
    pub fn to_string(&self) -> String {
        match self {
            CheckCartResult::Complete => t!("cart-result-complete"),
            CheckCartResult::Reduced => t!("cart-result-reduced"),
            CheckCartResult::Removed => t!("cart-result-removed"),
            CheckCartResult::Error(e) => t!("cart-result-error", error: e),
        }
    }
}

impl fmt::Display for CheckCartResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

// Enums

#[derive(Debug, Display, Clone, PartialEq, Eq, EnumIter, EnumString, Serialize, Deserialize)]
pub enum ProductForm {
    #[strum(to_string = "Ampoule")]
    Ampoule,
    #[strum(to_string = "Capsules")]
    Capsules,
    #[strum(to_string = "Container")]
    Container,
    #[strum(to_string = "DirectSpray")]
    DirectSpray,
    #[strum(to_string = "Multi")]
    Multi,
    #[strum(to_string = "Other")]
    Other,
    #[strum(to_string = "Solution")]
    Solution,
    #[strum(to_string = "VerticalSpray")]
    VerticalSpray,
    #[strum(to_string = "Vial")]
    Vial,
}

impl ProductForm {
    pub fn to_string(&self) -> String {
        match self {
            ProductForm::Ampoule => t!("product-form-ampoule"),
            ProductForm::Capsules => t!("product-form-capsules"),
            ProductForm::Container => t!("product-form-container"),
            ProductForm::DirectSpray => t!("product-form-direct-spray"),
            ProductForm::Multi => t!("product-form-multi"),
            ProductForm::Other => t!("product-form-other"),
            ProductForm::Solution => t!("product-form-solution"),
            ProductForm::VerticalSpray => t!("product-form-vertical-spray"),
            ProductForm::Vial => t!("product-form-vial"),
        }
    }

    pub fn to_frontend_string(&self) -> String {
        match self {
            ProductForm::Ampoule => t!("product-form-ampoule"),
            ProductForm::Capsules => t!("product-form-capsules"),
            ProductForm::Container => t!("product-form-container"),
            ProductForm::DirectSpray => t!("product-form-spray"),
            ProductForm::Multi => t!("product-form-multi"),
            ProductForm::Other => t!("product-form-other"),
            ProductForm::Solution => t!("product-form-solution"),
            ProductForm::VerticalSpray => t!("product-form-spray"),
            ProductForm::Vial => t!("product-form-vial"),
        }
    }
}

#[derive(Debug, Display, Clone, PartialEq, Eq, EnumIter, EnumString, Serialize, Deserialize)]
pub enum ProductVisibility {
    #[strum(to_string = "Private")]
    Private,
    #[strum(to_string = "Public")]
    Public,
    #[strum(to_string = "Unlisted")]
    Unlisted,
}

#[derive(Debug, Display, Clone, PartialEq, Eq, EnumIter, EnumString, Serialize, Deserialize)]
pub enum ProductPhase {
    #[strum(to_string = "Blue (Standard)")]
    Blue,
    #[strum(to_string = "Purple (Premium)")]
    Purple,
    #[strum(to_string = "Orange (SARMs)")]
    Orange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, EnumString, Serialize, Deserialize)]
pub enum Category {
    Chondrogenic,
    Osteogenic,
    Protective,
    Nootropic,
    Other,
}

impl Category {
    pub fn to_string(&self) -> String {
        match self {
            Category::Chondrogenic => t!("chondrogenic"),
            Category::Osteogenic => t!("osteogenic"),
            Category::Protective => t!("protective"),
            Category::Nootropic => t!("nootropic"),
            Category::Other => t!("other"),
        }
    }

    // Converts enum type to "key" type which is used in page handles etc.
    pub fn to_key(&self) -> &str {
        match self {
            Category::Chondrogenic => "chondrogenic",
            Category::Osteogenic => "osteogenic",
            Category::Protective => "protective",
            Category::Nootropic => "nootropic",
            Category::Other => "other",
        }
    }

    /// Converts a key string back to the corresponding Category variant
    pub fn from_key(s: &str) -> Option<Self> {
        match s {
            "chondrogenic" => Some(Category::Chondrogenic),
            "osteogenic" => Some(Category::Osteogenic),
            "protective" => Some(Category::Protective),
            "nootropic" => Some(Category::Nootropic),
            "other" => Some(Category::Other),
            _ => None,
        }
    }
}

// Manual Display implementation to use to_string
impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

// STOCK LOCATION ENTITIES

#[derive(Debug, Display, Clone, PartialEq, Eq, EnumIter, EnumString, Serialize, Deserialize)]
pub enum StockLocationShippingMethod {
    #[strum(to_string = "Manual")]
    Manual,
    #[strum(to_string = "Flat Rate")]
    FlatRate,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct StockLocation {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub shipping_method: StockLocationShippingMethod,
    // Flat rate cost in USD (used when shipping_method = FlatRate)
    pub flat_rate_usd: Option<f64>,
    // Country/city reference for this location
    pub country: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    // Per-item quantities at this location (populated when querying a specific location)
    pub quantities: Option<Vec<StockLocationQuantity>>,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct StockLocationQuantity {
    pub id: String,
    pub stock_item_id: String,
    pub stock_location_id: String,
    pub stock_location_name: Option<String>,
    pub quantity: i32,
    pub enabled: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    // Adjustment audit log for this location quantity
    pub adjustments: Option<Vec<StockQuantityAdjustment>>,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct StockQuantityAdjustment {
    pub id: String,
    pub stock_location_quantity_id: String,
    // Positive value = addition, negative value = subtraction
    pub delta: i32,
    // Required note explaining the adjustment
    pub note: String,
    // User ID of who made the adjustment (if available)
    pub adjusted_by: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Display, Clone, PartialEq, Eq, EnumIter, EnumString, Serialize, Deserialize)]
pub enum StockMode {
    #[strum(to_string = "Calculated")]
    // If the stock quantity of a product is calculated automatically
    Calculated,
    #[strum(to_string = "Force Stocked")]
    // If a product should be forced in stock (should avoid all stock calculations)
    ForceStocked,
    #[strum(to_string = "Force Unstocked")]
    // If a product should be forced out of stock (can't be purchased)
    ForceUnstocked,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct StockItem {
    // Unique stock item ID
    pub id: String,
    // Unique stock item SKU (must start with PBI or PBX e.g PBI0001)
    pub pbi_sku: String,
    // Reference name for this stock item
    pub name: String,
    // Description for this stock item (may contain notes)
    pub description: Option<String>,
    // Stock item thumbnail reference
    pub thumbnail_ref: Option<String>,
    // Estimated time of assembly per item (minutes)
    pub assembly_minutes: Option<i32>,
    // Default shipping time in days including synthesis/creation time
    pub default_shipping_days: Option<i32>,
    // Default cost in USD
    pub default_cost: Option<f64>,
    // Warning quantity in units (if total stock falls below this, item needs restocking)
    pub warning_quantity: Option<i32>,
    // Per-location stock quantities for this item
    pub location_quantities: Option<Vec<StockLocationQuantity>>,
    // When the stock item was created
    pub created_at: NaiveDateTime,
    // When the stock item was last updated
    pub updated_at: NaiveDateTime,
}

// Join table for Product Variant-StockItem relations
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct ProductVariantStockItemRelation {
    // Reference of the product variant
    pub product_variant_id: String,
    // The linked stock item
    pub stock_item_id: String,
    // Quantity of this stock item consumed per unit of the product variant
    pub quantity: i32,
}

#[derive(Debug, Display, Clone, PartialEq, Eq, EnumIter, EnumString, Serialize, Deserialize)]
pub enum DiscountType {
    #[strum(to_string = "Percentage")]
    Percentage,
    #[strum(to_string = "Fixed Amount")]
    FixedAmount,
    #[strum(to_string = "Percentage on shipping")]
    PercentageOnShipping,
    #[strum(to_string = "Fixed amount on shipping")]
    FixedAmountOnShipping,
}

// Discounts
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Discount {
    // UUID of this discount code
    pub id: String,
    // User-friendly code used for applying this discount at card (displayed in uppercase)
    pub code: String,
    // If this discount was created by affiliate, their associated id
    pub affiliate_id: Option<String>,
    // If this discount is active (default true), this is an override, expire_at, maximum_uses and this should be checked
    pub active: bool,
    // Type of discount
    pub discount_type: DiscountType,
    // If a Percentage type, the percentage off the cart total (excluding shipping) or the percentage off shipping total (for PercentageOnShipping)
    pub discount_percentage: Option<f64>,
    // If a discount amount type, the amount that should be removed from the cart total. If larger than cart total, reduce to 0.
    pub discount_amount: Option<f64>,
    // If a discount amount type, the amount of out of discount_amount that has been used (it can be re-applied at cart)
    pub amount_used: Option<f64>,
    // Maximum uses for this discount
    pub maximum_uses: Option<i32>,
    // How many times this discount has been used (default 0)
    pub discount_used: i32,
    // The active reduce quantity (used to reserve discount usage for open payments)
    pub active_reduce_quantity: i32,
    // List of valid countries this discount can be used in (only auto-applies in in these countries if this is defined)
    pub valid_countries: Option<Vec<String>>,
    // When this discount is valid if counting in amount of products
    pub valid_after_x_products: Option<i32>,
    // When this discount is valid if counting in cart total (excluding shipping)
    pub valid_after_x_total: Option<f64>,
    // Auto-apply (default false) -
    pub auto_apply: bool,

    // When this discount should expire (should be invalid past this date)
    pub expire_at: Option<NaiveDateTime>,
    // When this discount was created
    pub created_at: NaiveDateTime,
    // When this discount was last updated
    pub updated_at: NaiveDateTime,
}

// Affiliate stuff (not active for now)

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct AffiliateUser {
    // UUID of this affiliate
    pub id: String,
    // The level of the affiliate gives them certain perms
    pub level: i32,
    // The email of this affiliate
    pub email: String,
    // Where this affiliate is from
    pub country: String,
    // If this account is enabled and not restricted
    pub enabled: bool,
    // When this affiliate was created
    pub created_at: NaiveDateTime,
    // When this affiliate was last updated
    pub updated_at: NaiveDateTime,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct AffiliateWithdrawl {
    // UUID of this withdrawl
    pub id: String,
    // The ID of the affiliate that initiated this request
    pub affiliate_id: String,
    // The preferred crypto to withdraw into for this affiliate
    pub crypto: String,
    // The crypto address for this withdrawl
    pub crypto_address: String,
    // The transaction ID if this withdrawl request has been completed
    pub tx_id: Option<String>,
    // If this withdrawl has been completed (default false)
    pub completed: bool,
    // If this withdrawl has been cancelled for any misc reasons (default false)
    pub cancelled: bool,
    // When this withdrawl was completed
    pub completed_at: Option<NaiveDateTime>,
    // When this withdrawl was created
    pub created_at: NaiveDateTime,
    // When this withdrawl was last updated
    pub updated_at: NaiveDateTime,
}

// Error enum for discount validation
#[derive(Debug, Display, Clone, PartialEq, Eq, EnumIter, Serialize, Deserialize)]
pub enum DiscountValidationError {
    DiscountNotFound,
    DiscountInactive,
    DiscountExpired,
    MaximumUsesExceeded,
    AmountExceeded,
    CountryRequired,
    InvalidCountry,
    MinimumProductsRequired,
    MinimumTotalRequired,
    DatabaseError,
}

impl DiscountValidationError {
    pub fn to_string(&self) -> String {
        match self {
            DiscountValidationError::DiscountNotFound => t!("discount-code-not-found"),
            DiscountValidationError::DiscountInactive => t!("discount-code-not-active"),
            DiscountValidationError::DiscountExpired => t!("discount-code-expired"),
            DiscountValidationError::MaximumUsesExceeded => {
                t!("discount-code-max-uses")
            }
            DiscountValidationError::AmountExceeded => {
                t!("discount-amount-limit-exceeded")
            }
            DiscountValidationError::CountryRequired => t!("discount-needs-country"),
            DiscountValidationError::InvalidCountry => t!("discount-country-invalid"),
            DiscountValidationError::MinimumProductsRequired => t!("discount-minimum-products"),
            DiscountValidationError::MinimumTotalRequired => t!("discount-total-required"),
            DiscountValidationError::DatabaseError => {
                t!("discount-db-error")
            }
        }
    }
}

// Implement FromStr for DiscountValidationError (required by Dioxus server functions)
impl FromStr for DiscountValidationError {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "DiscountNotFound" => Ok(DiscountValidationError::DiscountNotFound),
            "DiscountInactive" => Ok(DiscountValidationError::DiscountInactive),
            "DiscountExpired" => Ok(DiscountValidationError::DiscountExpired),
            "MaximumUsesExceeded" => Ok(DiscountValidationError::MaximumUsesExceeded),
            "AmountExceeded" => Ok(DiscountValidationError::AmountExceeded),
            "CountryRequired" => Ok(DiscountValidationError::CountryRequired),
            "InvalidCountry" => Ok(DiscountValidationError::InvalidCountry),
            "DatabaseError" => Ok(DiscountValidationError::DatabaseError),
            _ => Err(format!("Unknown discount validation error: {}", s)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BasketUpdateResult {
    pub basket: CustomerBasket,
    pub discount_error: Option<DiscountValidationError>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomerShippingInfo {
    // The phone of the customer (added to tracking)
    pub phone: Option<String>,
    // If the customer joined the email push list
    pub email_list: bool,
    // Valid first name
    pub first_name: String,
    // Valid last name
    pub last_name: String,
    // The company name if exists
    pub company: Option<String>,
    // The primary address line
    pub address_line_1: String,
    // The second address line (e.g apt, suite, etc.)
    pub address_line_2: Option<String>,
    // The postcode for shipment
    pub post_code: String,
    // The province/state/county of shipment
    pub province: Option<String>,
    // The city of the shipment address
    pub city: String,
    // Do not set this on the frontend, this will be implied by the cart automatically
    pub country: Option<String>,
}

#[derive(Debug, Display, Clone, PartialEq, Eq, EnumIter, Serialize, Deserialize)]
pub enum PaymentStatus {
    Cancelled,
    Failed,
    Paid,
    Pending,
    Refunded,
    Expired,
}

impl PaymentStatus {
    pub fn to_string(&self) -> String {
        match self {
            PaymentStatus::Cancelled => t!("payment-status-cancelled"),
            PaymentStatus::Failed => t!("payment-status-failed"),
            PaymentStatus::Paid => t!("payment-status-paid"),
            PaymentStatus::Pending => t!("payment-status-pending"),
            PaymentStatus::Refunded => t!("payment-status-refunded"),
            PaymentStatus::Expired => t!("payment-status-expired"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaymentShortInfo {
    // The status of this payment
    pub status: PaymentStatus,
    // The order linked to this payment
    pub order_id: String,
    // The reference code for the order
    pub order_ref_code: String,
    // The URL to visit for payment
    pub processor_url: String,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct OrderShortInfo {
    pub ref_code: String,
    pub shipping_option: ShippingOption,
    pub billing_country: String,
    pub tracking_url: Option<String>,
    pub total_amount_usd: f64,
    pub paid: bool,
    pub created_at: Option<NaiveDateTime>,
    pub paid_at: Option<NaiveDateTime>,
    pub prepared_at: Option<NaiveDateTime>,
    pub fulfilled_at: Option<NaiveDateTime>,
    pub items: Vec<OrderShortItem>,
    pub pre_orders: Vec<ShortPreOrder>,
    pub contains_back_order: bool,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct OrderShortItem {
    pub id: String,
    pub product_variant_id: String,
    pub quantity: i32,
    pub price_usd: f64,
    pub product_title: String,
    pub variant_name: String,
    pub pre_order_on_purchase: bool,
}

// USED IN ADMIN ROUTES
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct OrderInfo {
    pub id: String,
    pub ref_code: String,
    pub customer_id: Option<String>,
    pub customer_email: String,
    pub add_to_email_list: bool,
    pub billing_country: String,
    pub shipping_option: ShippingOption,
    pub subtotal_usd: f64,
    pub shipping_usd: f64,
    pub order_weight: f64,
    pub refund_comment: Option<String>,
    pub status: OrderStatus,
    pub fulfilled_at: Option<NaiveDateTime>,
    pub cancelled_at: Option<NaiveDateTime>,
    pub refunded_at: Option<NaiveDateTime>,
    pub prepared_at: Option<NaiveDateTime>,
    pub tracking_url: Option<String>,
    pub total_amount_usd: f64,
    pub discount_id: Option<String>,
    pub notes: Option<String>,
    pub address: Option<CustomerShippingInfo>,
    pub items: Vec<OrderShortItem>,
    pub backorder_reduces: Vec<BackOrPreOrderActiveReduce>,
    pub preorder_reduces: Vec<BackOrPreOrderActiveReduce>,
    pub pre_orders: Vec<PreOrder>,
    pub payments: Vec<PaymentInfo>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct BackOrPreOrderActiveReduce {
    pub id: String,
    pub order_id: String,
    pub order_item_id: String,
    pub stock_item_id: String,
    pub reduction_quantity: i32,
    pub active: bool,
    pub stock_location_id: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct PaymentInfo {
    pub id: String,
    pub method: String,
    pub processor_ref: Option<String>,
    pub processor_url: Option<String>,
    pub status: PaymentStatus,
    pub amount_usd: f64,
    pub paid_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Display, Clone, PartialEq, Eq, EnumIter, EnumString, Serialize, Deserialize)]
pub enum OrderStatus {
    #[strum(to_string = "Cancelled")]
    Cancelled,
    #[strum(to_string = "Fulfilled")]
    Fulfilled,
    #[strum(to_string = "Paid")]
    Paid,
    #[strum(to_string = "Pending")]
    Pending,
    // TO DEPRECATE?
    #[strum(to_string = "Pending")]
    Processing,
    #[strum(to_string = "Refunded")]
    Refunded,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct PreOrder {
    pub id: String,
    pub order_item_id: String,
    pub parent_order_id: String,
    pub add_to_email_list: bool,
    pub shipping_option: ShippingOption,
    pub pre_order_weight: f64,
    pub fulfilled_at: Option<NaiveDateTime>,
    pub prepared_at: Option<NaiveDateTime>,
    pub tracking_url: Option<String>,
    pub notes: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct ShortPreOrder {
    pub id: String,
    pub order_item_id: String,
    pub parent_order_id: String,
    pub shipping_option: ShippingOption,
    pub fulfilled_at: Option<NaiveDateTime>,
    pub prepared_at: Option<NaiveDateTime>,
    pub tracking_url: Option<String>,
}

// Store Settings
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct StoreSettingsInfo {
    pub lock_store: bool,
    pub lock_comment: Option<String>,
}

// For blogs
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct BlogPost {
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub thumbnail_url: Option<String>,
    pub blog_md: String,
    pub posted_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
