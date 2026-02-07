use crate::Route;
use crate::utils::countries::allowed_countries;
use dioxus::prelude::*;
use dioxus_i18n::t;

#[component]
pub fn ShippingPolicy() -> Element {
    rsx! {
         document::Title { { format!("{} - {}", t!("brand"), t!("shipping-info") ) } }

         div { class: "py-6 md:py-12 flex justify-center",
             div {
                 class: "max-w-[1000px] w-full",
                 h2 {
                     class: "mb-4",
                     {t!("shipping-info")}
                 }
                 div {
                     p {
                         class: "para",
                         {t!("shipping-line-1", num: allowed_countries().len()) }
                     },
                     p {
                         class: "para",
                         {t!("shipping-line-2")}
                     }
                     p {
                         class: "para",
                         {t!("shipping-line-3")}
                     }
                     p {
                         class: "para",
                         {t!("shipping-line-4")}
                     }
                     p {
                         class: "para mt-2",
                         span {
                             {format!("{} ", t!("if-any-more-questions"))}
                             Link {
                                 class: "a",
                                 to: Route::Faq {},
                                 { t!("faq") }
                             }
                         }
                     }
                 }
             }
         }
    }
}
