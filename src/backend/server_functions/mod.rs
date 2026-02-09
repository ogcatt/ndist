// src/backend/server_functions/mod.rs

pub mod auth;
pub mod basket;
pub mod blog;
pub mod discounts;
pub mod inventory;
pub mod orders;
pub mod products;
pub mod stock_calculations;
pub mod uploads;

// Re-export all public items to maintain backward compatibility
pub use auth::*;
pub use basket::*;
pub use blog::*;
pub use discounts::*;
pub use inventory::*;
pub use orders::*;
pub use products::*;
pub use stock_calculations::*;
pub use uploads::*;

// Allow payments access with server_functions::payments
pub use super::payments;
