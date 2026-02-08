//! The components module contains all shared components for our app. Components are the building blocks of dioxus apps.
//! They can be used to defined common UI elements like buttons, forms, and modals. In this template, we define a Hero
//! component and an Echo component for fullstack apps to be used in our app.

pub mod navbar;
pub use navbar::Header;

pub mod footer;
pub use footer::Footer;

mod header_footer;
pub use header_footer::HeaderFooter;

mod admin;
pub use admin::AdminWrapper;

mod common;
pub use common::*;

mod product_card;
pub use product_card::*;

mod collections_grid;
pub use collections_grid::*;

mod checkout_navbar;
pub use checkout_navbar::*;

mod shipping_map;
pub use shipping_map::*;

mod i18n_popup;
pub use i18n_popup::*;

mod smiles_viewer;
pub use smiles_viewer::*;

mod seo;
pub use seo::*;

mod search_results;
pub use search_results::*;

mod account_popup;
pub use account_popup::*;
