use crate::Route;
use dioxus::prelude::*;
use dioxus_i18n::t;
use crate::backend::front_entities::*;
use crate::backend::server_functions::{check_payment, delete_payment};
use gloo_timers::future::TimeoutFuture;

#[component]
pub fn CheckoutPayment(payment_id: String) -> Element {
    let mut timer = use_signal(|| 0i32);
    let navigator = use_navigator();

    use_coroutine(move |_: UnboundedReceiver<()>| async move {
        loop {
            TimeoutFuture::new(60_000).await;
            timer.set(timer() + 1);
        }
    });

    let payment_info = use_resource({
        let payment_id = payment_id.clone();
        move || {
            let _ = timer();
            let value = payment_id.clone();
            async move {
                check_payment(value).await
            }
        }
    });

    let delete_payment = {
        let payment_id = payment_id.clone();
        let mut timer = timer.clone();
        move || async move {
            delete_payment(payment_id).await;
            timer.set(timer() + 1); // Get payment status again
        }
    };

    use_effect(move || {
        match &*payment_info.read_unchecked() {
            Some(Ok(result)) => {
                tracing::info!("Payment data: {result:#?}");
                match result.status {
                    PaymentStatus::Paid => {
                        navigator.push(Route::OrderStatus { order_id: result.order_id.clone() });
                    },
                    PaymentStatus::Refunded => {
                        navigator.push(Route::Cart {});
                    },
                    _ => {}
                }
            },
            Some(Err(e)) => {
                tracing::info!("Payment error: {e:#?}");
            },
            None => {}
        }
    });

    rsx! {
        document::Title { { format!("{} - {}", t!("brand"), t!("make-your-payment") ) } }

        div {
            class: "min-h-screen py-2 md:py-8",
            div {
                class: "max-w-2xl mx-auto px-4 sm:px-6 lg:px-8",

                // Header Card
                div {
                    class: "mb-2",
                    div {
                        class: "py-5",
                        h1 {
                            class: "text-2xl text-gray-900",
                            { t!("payment") }
                        }
                    }
                }

                // Main Content Card
                div {
                    class: "bg-white rounded-lg border border-gray-200 overflow-hidden",
                    div {
                        class: "px-6 py-8",

                        {
                            match &*payment_info.read_unchecked() {
                            Some(Ok(result)) => {
                                match result.status {
                                    PaymentStatus::Pending => rsx! {
                                        div {
                                            class: "text-center",
                                            // Loading spinner
                                            div {
                                                class: "flex justify-center mb-6",
                                                div {
                                                    class: "w-12 h-12 border-4 border-blue-200 border-t-blue-600 rounded-full animate-spin"
                                                }
                                            }

                                            // Order reference
                                            /*
                                            div {
                                                class: "mb-6",
                                                p {
                                                    class: "text-sm font-medium text-gray-500 mb-1",
                                                    "Order Reference"
                                                }
                                                p {
                                                    class: "text-xl font-mono font-semibold text-gray-900 bg-gray-100 rounded-md px-3 py-2 inline-block",
                                                    "{result.order_ref_code}"
                                                }
                                            }
                                            */

                                            // Status badge
                                            div {
                                                class: "mb-6",
                                                span {
                                                    class: "inline-flex items-center px-3 py-1 rounded-full text-sm font-medium bg-yellow-100 text-yellow-800 border border-yellow-200",
                                                    div {
                                                        class: "w-2 h-2 bg-yellow-400 rounded-full mr-2 animate-pulse"
                                                    }
                                                    { t!("payment-pending-for", ref: result.order_ref_code.clone()) }
                                                }
                                            }

                                            // Payment instructions
                                            div {
                                                class: "bg-blue-50 rounded-md p-4 mb-6",
                                                p {
                                                    class: "text-sm text-blue-800",
                                                    { t!("please-complete-payment") }
                                                }
                                            }

                                            // Payment button
                                            a {
                                                class: "inline-block mb-4",
                                                href: result.processor_url.clone(),
                                                target: "_blank",
                                                button {
                                                    class: "inline-flex justify-center items-center px-6 py-3 border border-transparent text-base font-medium rounded-md text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 transition-all duration-200 shadow-sm",
                                                    svg {
                                                        class: "w-5 h-5 mr-2",
                                                        fill: "none",
                                                        stroke: "currentColor",
                                                        view_box: "0 0 24 24",
                                                        path {
                                                            stroke_linecap: "round",
                                                            stroke_linejoin: "round",
                                                            stroke_width: "2",
                                                            d: "M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"
                                                        }
                                                    }
                                                    { t!("complete-payment") }
                                                }
                                            }

                                            // Auto-refresh indicator
                                            p {
                                                class: "text-xs text-gray-500",
                                                { t!("checking-payment-status") }
                                            }

                                            p {
                                                class: "text-xs text-red-500 mt-2 cursor-pointer",
                                                onclick: move |_| {
                                                    spawn({
                                                        let delete_payment = delete_payment.clone();
                                                        async move {delete_payment().await;}
                                                    });
                                                },
                                                { t!("delete-payment") }
                                            }
                                        }
                                    },

                                    PaymentStatus::Failed => rsx! {
                                        div {
                                            class: "text-center",
                                            // Error icon
                                            div {
                                                class: "flex justify-center mb-6",
                                                div {
                                                    class: "w-12 h-12 bg-red-100 rounded-full flex items-center justify-center",
                                                    svg {
                                                        class: "w-6 h-6 text-red-600",
                                                        fill: "none",
                                                        stroke: "currentColor",
                                                        view_box: "0 0 24 24",
                                                        path {
                                                            stroke_linecap: "round",
                                                            stroke_linejoin: "round",
                                                            stroke_width: "2",
                                                            d: "M6 18L18 6M6 6l12 12"
                                                        }
                                                    }
                                                }
                                            }

                                            // Order reference

                                            /*
                                            div {
                                                class: "mb-6",
                                                p {
                                                    class: "text-sm font-medium text-gray-500 mb-1",
                                                    "Order Reference"
                                                }
                                                p {
                                                    class: "text-xl font-mono font-semibold text-gray-900 bg-gray-100 rounded-md px-3 py-2 inline-block",
                                                    "{result.order_ref_code}"
                                                }
                                            }
                                            */

                                            // Status
                                            div {
                                                class: "mb-6",
                                                span {
                                                    class: "inline-flex items-center px-3 py-1 rounded-full text-sm font-medium bg-red-100 text-red-800 border border-red-200",
                                                    { t!("payment-failed") }
                                                }
                                            }

                                            // Error message
                                            div {
                                                class: "bg-red-50 border border-red-200 rounded-md p-4 mb-6",
                                                p {
                                                    class: "text-sm text-red-800 font-medium mb-1",
                                                    { t!("payment-could-not-be-processed") }
                                                }
                                                p {
                                                    class: "text-sm text-red-700",
                                                    { t!("please-try-again") }
                                                }
                                            }

                                            // Return home button
                                            Link {
                                                to: Route::Home {},
                                                class: "inline-block",
                                                button {
                                                    class: "inline-flex justify-center items-center px-6 py-3 border border-transparent text-base font-medium rounded-md text-white bg-gray-600 hover:bg-gray-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-gray-500 transition-colors duration-200",
                                                    svg {
                                                        class: "w-5 h-5 mr-2",
                                                        fill: "none",
                                                        stroke: "currentColor",
                                                        view_box: "0 0 24 24",
                                                        path {
                                                            stroke_linecap: "round",
                                                            stroke_linejoin: "round",
                                                            stroke_width: "2",
                                                            d: "M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6"
                                                        }
                                                    }
                                                    { t!("return-to-home") }
                                                }
                                            }
                                        }
                                    },

                                    PaymentStatus::Cancelled => rsx! {
                                        div {
                                            class: "text-center",
                                            // Cancel icon
                                            div {
                                                class: "flex justify-center mb-6",
                                                div {
                                                    class: "w-12 h-12 bg-gray-100 rounded-full flex items-center justify-center",
                                                    svg {
                                                        class: "w-6 h-6 text-gray-600",
                                                        fill: "none",
                                                        stroke: "currentColor",
                                                        view_box: "0 0 24 24",
                                                        path {
                                                            stroke_linecap: "round",
                                                            stroke_linejoin: "round",
                                                            stroke_width: "2",
                                                            d: "M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728L5.636 5.636m12.728 12.728L18.364 5.636M5.636 18.364l12.728-12.728"
                                                        }
                                                    }
                                                }
                                            }

                                            // Status
                                            div {
                                                class: "mb-6",
                                                span {
                                                    class: "inline-flex items-center px-3 py-1 rounded-full text-sm font-medium bg-gray-100 text-gray-800 border border-gray-200",
                                                    { t!("payment-cancelled") }
                                                }
                                            }

                                            // Info message
                                            div {
                                                class: "bg-gray-50 border border-gray-200 rounded-md p-4 mb-6",
                                                p {
                                                    class: "text-sm text-gray-700",
                                                    { t!("payment-cancelled-info") }
                                                }
                                            }

                                            // Action buttons
                                            div {
                                                class: "space-y-3",
                                                Link {
                                                    to: Route::Cart {},
                                                    class: "block",
                                                    button {
                                                        class: "w-full inline-flex justify-center items-center px-6 py-3 border border-transparent text-base font-medium rounded-md text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 transition-colors duration-200",
                                                        { t!("return-to-cart") }
                                                    }
                                                }
                                                Link {
                                                    to: Route::Home {},
                                                    class: "block",
                                                    button {
                                                        class: "w-full inline-flex justify-center items-center px-6 py-3 border border-gray-300 text-base font-medium rounded-md text-gray-700 bg-white hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 transition-colors duration-200",
                                                        { t!("continue-shopping") }
                                                    }
                                                }
                                            }
                                        }
                                    },

                                    PaymentStatus::Expired => rsx! {
                                        div {
                                            class: "mb-6",
                                            span {
                                                class: "inline-flex items-center px-3 py-1 rounded-full text-sm font-medium bg-gray-100 text-gray-800 border border-gray-200",
                                                { t!("payment-expired") }
                                            }
                                        }

                                        // Info message
                                        div {
                                            class: "bg-gray-50 border border-gray-200 rounded-md p-4 mb-6",
                                            p {
                                                class: "text-sm text-gray-700",
                                                { t!("payment-expired-info") }
                                            }
                                        }

                                        // Action buttons
                                        div {
                                            class: "space-y-3",
                                            Link {
                                                to: Route::Cart {},
                                                class: "block",
                                                button {
                                                    class: "w-full inline-flex justify-center items-center px-6 py-3 border border-transparent text-base font-medium rounded-md text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 transition-colors duration-200",
                                                    { t!("return-to-cart") }
                                                }
                                            }
                                            Link {
                                                to: Route::Home {},
                                                class: "block",
                                                button {
                                                    class: "w-full inline-flex justify-center items-center px-6 py-3 border border-gray-300 text-base font-medium rounded-md text-gray-700 bg-white hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 transition-colors duration-200",
                                                    { t!("continue-shopping") }
                                                }
                                            }
                                        }
                                    },

                                    _ => rsx! {
                                        div {
                                            class: "text-center",
                                            p {
                                                class: "text-gray-600",
                                                "Unexpected payment status: {result.status:?}"
                                            }
                                        }
                                    }
                                }
                            },

                            Some(Err(e)) => rsx! {
                                div {
                                    class: "text-center",
                                    // Error icon
                                    div {
                                        class: "flex justify-center mb-6",
                                        div {
                                            class: "w-12 h-12 bg-red-100 rounded-full flex items-center justify-center",
                                            svg {
                                                class: "w-6 h-6 text-red-600",
                                                fill: "none",
                                                stroke: "currentColor",
                                                view_box: "0 0 24 24",
                                                path {
                                                    stroke_linecap: "round",
                                                    stroke_linejoin: "round",
                                                    stroke_width: "2",
                                                    d: "M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z"
                                                }
                                            }
                                        }
                                    }

                                    h2 {
                                        class: "text-lg font-semibold text-gray-900 mb-2",
                                        { t!("unable-to-load-payment") }
                                    }

                                    div {
                                        class: "bg-red-50 border border-red-200 rounded-md p-4 mb-6",
                                        p {
                                            class: "text-sm text-red-800",
                                            {format!("Error: {:?}", e)}
                                        }
                                    }

                                    Link {
                                        to: Route::Home {},
                                        button {
                                            class: "inline-flex justify-center items-center px-6 py-3 border border-transparent text-base font-medium rounded-md text-white bg-gray-600 hover:bg-gray-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-gray-500 transition-colors duration-200",
                                            { t!("return-to-home") }
                                        }
                                    }
                                }
                            },

                            None => rsx! {
                                // Skeleton loading state
                                div {
                                    class: "text-center",

                                    // Loading skeleton for order reference
                                    div {
                                        class: "mb-6",
                                        p {
                                            class: "text-sm font-medium text-gray-500 mb-1",
                                            { t!("order-reference") }
                                        }
                                        div {
                                            class: "skeleton mx-auto",
                                            style: "width: 8rem; height: 2.5rem; border-radius: 0.375rem;"
                                        }
                                    }

                                    // Loading skeleton for status
                                    div {
                                        class: "mb-6",
                                        div {
                                            class: "skeleton mx-auto",
                                            style: "width: 7rem; height: 1.75rem; border-radius: 9999px;"
                                        }
                                    }

                                    // Loading skeleton for button
                                    div {
                                        class: "mb-6",
                                        div {
                                            class: "skeleton mx-auto",
                                            style: "width: 10rem; height: 2.75rem; border-radius: 0.375rem;"
                                        }
                                    }

                                    p {
                                        class: "text-sm text-gray-500",
                                        { t!("loading-payment") }
                                    }
                                }
                            }
                        }
                        }
                    }
                }
            }
        }
    }
}
