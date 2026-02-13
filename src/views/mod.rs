//! The views module contains the components for all Layouts and Routes for our app. Each layout and route in our [`Route`]
//! enum will render one of these components.
//!
//!
//! The [`Home`] and [`Blog`] components will be rendered when the current route is [`Route::Home`] or [`Route::Blog`] respectively.
//!
//!
//! The [`Navbar`] component will be rendered on all pages of our app since every page is under the layout. The layout defines
//! a common wrapper around all child routes.

mod home;
pub use home::Home;

mod blog;
pub use blog::BlogPostPage;
pub use blog::BlogPosts;

mod collections;
pub use collections::*;

pub mod admin;
pub use admin::*;

pub mod products;
pub use products::*;

pub mod contact;
pub use contact::*;

pub mod faq;
pub use faq::*;

pub mod policies;
pub use policies::*;

pub mod shipping;
pub use shipping::*;

pub mod peptide_calculator;
pub use peptide_calculator::*;

pub mod not_found;
pub use not_found::*;

pub mod about;
pub use about::*;

pub mod cart;
pub use cart::*;

pub mod checkout;
pub use checkout::*;

pub mod checkout_payment;
pub use checkout_payment::*;

pub mod order_status;
pub use order_status::*;

mod dashboard;
pub use dashboard::*;

mod groups;
pub use groups::*;
