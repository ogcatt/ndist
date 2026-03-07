#![allow(non_snake_case)]

use chrono::{Duration, Local};
use dioxus::prelude::*;
use std::collections::BTreeMap;

use crate::Route;
use crate::backend::front_entities::*;
use crate::backend::server_functions;

#[component]
pub fn Dashboard() -> Element {
    let orders = use_resource(|| async move { server_functions::admin_get_orders(false).await });
    let products = use_resource(|| async move { server_functions::admin_get_products(false).await });
    let stock_items = use_resource(|| async move { server_functions::admin_get_stock_items().await });
    let current_user = use_resource(|| server_functions::get_current_user());

    // Render Chart.js charts when order data arrives
    use_effect(move || {
        let js = {
            let orders_ref = orders.read();
            if let Some(Ok(order_list)) = orders_ref.as_ref() {
                let now = Local::now().naive_local();
                let cutoff = now - Duration::days(30);

                let mut daily_orders: BTreeMap<String, i32> = BTreeMap::new();
                let mut daily_revenue: BTreeMap<String, f64> = BTreeMap::new();
                // Pre-fill all 30 days with zero so gaps show
                for i in 0..30i64 {
                    let day = now.date() - Duration::days(29 - i);
                    let key = day.format("%b %d").to_string();
                    daily_orders.insert(key.clone(), 0);
                    daily_revenue.insert(key, 0.0);
                }
                for order in order_list.iter().filter(|o| o.created_at >= cutoff) {
                    let key = order.created_at.date().format("%b %d").to_string();
                    *daily_orders.entry(key.clone()).or_insert(0) += 1;
                    if order.status != OrderStatus::Cancelled && order.status != OrderStatus::Refunded {
                        *daily_revenue.entry(key).or_insert(0.0) += order.total_amount_usd;
                    }
                }

                let labels_js = format!(
                    "[{}]",
                    daily_orders.keys().map(|k| format!("\"{}\"", k)).collect::<Vec<_>>().join(",")
                );
                let orders_js = format!(
                    "[{}]",
                    daily_orders.values().map(|v| v.to_string()).collect::<Vec<_>>().join(",")
                );
                let revenue_js = format!(
                    "[{}]",
                    daily_revenue.values().map(|v| format!("{:.2}", v)).collect::<Vec<_>>().join(",")
                );

                Some(format!(
                    r#"
(function() {{
    function tryRender() {{
        if (typeof Chart === 'undefined') {{ setTimeout(tryRender, 200); return; }}
        var labels = {labels};
        var orderData = {orders};
        var revenueData = {revenue};
        var axisFont = {{ font: {{ family: 'monospace', size: 10 }} }};

        var oc = document.getElementById('admin-orders-chart');
        if (oc) {{
            if (oc._chart) oc._chart.destroy();
            oc._chart = new Chart(oc.getContext('2d'), {{
                type: 'bar',
                data: {{
                    labels: labels,
                    datasets: [{{ label: 'Orders', data: orderData, backgroundColor: 'rgba(0,0,0,0.8)', borderRadius: 2 }}]
                }},
                options: {{
                    responsive: true, maintainAspectRatio: false,
                    plugins: {{ legend: {{ display: false }} }},
                    scales: {{
                        x: {{ grid: {{ display: false }}, ticks: Object.assign({{ maxTicksLimit: 8 }}, axisFont) }},
                        y: {{ beginAtZero: true, ticks: Object.assign({{ stepSize: 1 }}, axisFont), grid: {{ color: 'rgba(0,0,0,0.06)' }} }}
                    }}
                }}
            }});
        }}

        var rc = document.getElementById('admin-revenue-chart');
        if (rc) {{
            if (rc._chart) rc._chart.destroy();
            rc._chart = new Chart(rc.getContext('2d'), {{
                type: 'line',
                data: {{
                    labels: labels,
                    datasets: [{{ label: 'Revenue ($)', data: revenueData, borderColor: 'rgba(0,0,0,0.85)', backgroundColor: 'rgba(0,0,0,0.05)', fill: true, tension: 0.3, pointRadius: 2, borderWidth: 2 }}]
                }},
                options: {{
                    responsive: true, maintainAspectRatio: false,
                    plugins: {{ legend: {{ display: false }} }},
                    scales: {{
                        x: {{ grid: {{ display: false }}, ticks: Object.assign({{ maxTicksLimit: 8 }}, axisFont) }},
                        y: {{ beginAtZero: true, ticks: Object.assign({{ callback: function(v) {{ return '$' + v.toFixed(0); }} }}, axisFont), grid: {{ color: 'rgba(0,0,0,0.06)' }} }}
                    }}
                }}
            }});
        }}
    }}
    tryRender();
}})();"#,
                    labels = labels_js,
                    orders = orders_js,
                    revenue = revenue_js
                ))
            } else {
                None
            }
        };
        if let Some(js) = js {
            document::eval(&js);
        }
    });

    // --- Compute stats (reads subscribe component to signal changes) ---
    let now = Local::now().naive_local();
    let cutoff_30 = now - Duration::days(30);
    let cutoff_7 = now - Duration::days(7);

    let (orders_30d, orders_7d, revenue_30d, unfulfilled, backordered, recent_orders) = {
        let ref_ = orders.read();
        if let Some(Ok(list)) = ref_.as_ref() {
            let o30: Vec<_> = list.iter().filter(|o| o.created_at >= cutoff_30).collect();
            let o7 = list.iter().filter(|o| o.created_at >= cutoff_7).count();
            let rev: f64 = o30
                .iter()
                .filter(|o| o.status != OrderStatus::Cancelled && o.status != OrderStatus::Refunded)
                .map(|o| o.total_amount_usd)
                .sum();
            let unf = list
                .iter()
                .filter(|o| {
                    matches!(o.status, OrderStatus::Paid | OrderStatus::Pending | OrderStatus::Processing)
                        && o.fulfilled_at.is_none()
                })
                .count();
            let back = list.iter().filter(|o| !o.backorder_reduces.is_empty()).count();
            let mut sorted = list.clone();
            sorted.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            let recent: Vec<OrderInfo> = sorted.into_iter().take(8).collect();
            (o30.len(), o7, rev, unf, back, recent)
        } else {
            (0usize, 0usize, 0.0f64, 0usize, 0usize, vec![])
        }
    };

    let (total_products, public_products, preorder_products, recent_products) = {
        let ref_ = products.read();
        if let Some(Ok(list)) = ref_.as_ref() {
            let public = list.iter().filter(|p| p.visibility == ProductVisibility::Public).count();
            let pre = list.iter().filter(|p| p.pre_order).count();
            let mut sorted = list.clone();
            sorted.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
            let recent: Vec<Product> = sorted.into_iter().take(5).collect();
            (list.len(), public, pre, recent)
        } else {
            (0usize, 0usize, 0usize, vec![])
        }
    };

    let (low_stock_count, total_stock, low_stock_items) = {
        let ref_ = stock_items.read();
        if let Some(Ok(list)) = ref_.as_ref() {
            let low: Vec<StockItem> = list
                .iter()
                .filter(|i| {
                    if let Some(warn) = i.warning_quantity {
                        let total: i32 = i.location_quantities.as_deref().unwrap_or(&[]).iter().map(|lq| lq.quantity).sum();
                        total < warn
                    } else {
                        false
                    }
                })
                .take(6)
                .cloned()
                .collect();
            (low.len(), list.len(), low)
        } else {
            (0usize, 0usize, vec![])
        }
    };

    let orders_loading = orders.read().is_none();

    rsx! {
        div {
            class: "space-y-5",

            // ── Header ──────────────────────────────────────────────────────
            div {
                class: "flex items-start justify-between border-b border-gray-200 pb-4",
                div {
                    h1 { class: "text-2xl font-bold text-gray-900", "Overview" }
                    p { class: "mt-1 text-sm text-gray-500", "Admin dashboard" }
                }
                match current_user.read().as_ref() {
                    Some(Ok(Some(user))) => rsx! {
                        div { class: "text-right",
                            p { class: "text-sm font-medium text-gray-900", "{user.name}" }
                            p { class: "text-xs text-gray-500 mt-0.5", "{user.email}" }
                            span { class: "inline-block mt-1 px-2 py-0.5 bg-black text-white text-xs font-mono", "Administrator" }
                        }
                    },
                    _ => rsx! {}
                }
            }

            // ── Stat cards ──────────────────────────────────────────────────
            div {
                class: "grid grid-cols-2 lg:grid-cols-4 gap-3",

                Link { to: Route::AdminOrders {},
                    div {
                        class: "bg-white border border-gray-200 p-4 hover:border-gray-400 transition-colors cursor-pointer",
                        p { class: "text-xs font-medium text-gray-500 uppercase tracking-wide", "Orders (30d)" }
                        p { class: "text-2xl font-bold text-gray-900 mt-1 font-mono",
                            if orders_loading { "—" } else { "{orders_30d}" }
                        }
                        p { class: "text-xs text-gray-400 mt-1", "{orders_7d} this week" }
                    }
                }

                Link { to: Route::AdminOrders {},
                    div {
                        class: "bg-white border border-gray-200 p-4 hover:border-gray-400 transition-colors cursor-pointer",
                        p { class: "text-xs font-medium text-gray-500 uppercase tracking-wide", "Revenue (30d)" }
                        p { class: "text-2xl font-bold text-gray-900 mt-1 font-mono",
                            if orders_loading { "—" } else { "${revenue_30d:.0}" }
                        }
                        p { class: "text-xs text-gray-400 mt-1", "Excl. cancelled / refunded" }
                    }
                }

                Link { to: Route::AdminOrders {},
                    div {
                        class: format!(
                            "p-4 border hover:border-gray-400 transition-colors cursor-pointer {}",
                            if unfulfilled > 0 { "bg-yellow-50 border-yellow-200" } else { "bg-white border-gray-200" }
                        ),
                        p { class: "text-xs font-medium text-gray-500 uppercase tracking-wide", "Unfulfilled" }
                        p {
                            class: format!(
                                "text-2xl font-bold mt-1 font-mono {}",
                                if unfulfilled > 0 { "text-yellow-700" } else { "text-gray-900" }
                            ),
                            if orders_loading { "—" } else { "{unfulfilled}" }
                        }
                        p { class: "text-xs text-gray-400 mt-1", "{backordered} backordered" }
                    }
                }

                Link { to: Route::AdminInventory {},
                    div {
                        class: format!(
                            "p-4 border hover:border-gray-400 transition-colors cursor-pointer {}",
                            if low_stock_count > 0 { "bg-orange-50 border-orange-200" } else { "bg-white border-gray-200" }
                        ),
                        p { class: "text-xs font-medium text-gray-500 uppercase tracking-wide", "Low Stock" }
                        p {
                            class: format!(
                                "text-2xl font-bold mt-1 font-mono {}",
                                if low_stock_count > 0 { "text-orange-700" } else { "text-gray-900" }
                            ),
                            "{low_stock_count}"
                        }
                        p { class: "text-xs text-gray-400 mt-1", "{total_stock} total items" }
                    }
                }
            }

            // ── Charts ──────────────────────────────────────────────────────
            div {
                class: "grid grid-cols-1 lg:grid-cols-2 gap-4",
                div {
                    class: "bg-white border border-gray-200 p-4",
                    p { class: "text-xs font-bold text-gray-700 uppercase tracking-wide mb-3", "Orders — Last 30 Days" }
                    div { class: "h-44",
                        canvas { id: "admin-orders-chart" }
                    }
                }
                div {
                    class: "bg-white border border-gray-200 p-4",
                    p { class: "text-xs font-bold text-gray-700 uppercase tracking-wide mb-3", "Revenue — Last 30 Days (USD)" }
                    div { class: "h-44",
                        canvas { id: "admin-revenue-chart" }
                    }
                }
            }

            // ── Main content row ────────────────────────────────────────────
            div {
                class: "grid grid-cols-1 lg:grid-cols-3 gap-4",

                // Recent orders (2 cols)
                div {
                    class: "lg:col-span-2 bg-white border border-gray-200",
                    div {
                        class: "px-4 py-3 border-b border-gray-200 flex items-center justify-between",
                        p { class: "text-xs font-bold text-gray-700 uppercase tracking-wide", "Recent Orders" }
                        Link { to: Route::AdminOrders {}, class: "text-xs text-gray-500 hover:text-gray-900 transition-colors", "View all →" }
                    }
                    if orders_loading {
                        p { class: "px-4 py-5 text-sm text-gray-400 italic", "Loading..." }
                    } else if recent_orders.is_empty() {
                        p { class: "px-4 py-5 text-sm text-gray-500 text-center", "No orders yet" }
                    } else {
                        div {
                            for order in recent_orders {
                                RecentOrderRow { order }
                            }
                        }
                    }
                }

                // Right column
                div {
                    class: "space-y-4",

                    // Quick access
                    div {
                        class: "bg-white border border-gray-200",
                        div { class: "px-4 py-3 border-b border-gray-200",
                            p { class: "text-xs font-bold text-gray-700 uppercase tracking-wide", "Quick Access" }
                        }
                        div {
                            Link { to: Route::AdminOrders {},
                                class: "flex items-center gap-3 px-4 py-2.5 hover:bg-gray-50 transition-colors border-b border-gray-100",
                                img { class: "w-4 h-4 opacity-50", src: asset!("/assets/icons/file-tray-stacked.svg") }
                                span { class: "flex-1 text-sm text-gray-700", "Orders" }
                                if unfulfilled > 0 {
                                    span {
                                        class: "min-w-5 h-5 bg-red-500 text-white text-xs rounded-full flex items-center justify-center font-medium px-1",
                                        "{unfulfilled}"
                                    }
                                } else {
                                    span { class: "text-xs text-gray-500 font-mono", "{orders_30d} / 30d" }
                                }
                            }
                            Link { to: Route::AdminProducts {},
                                class: "flex items-center gap-3 px-4 py-2.5 hover:bg-gray-50 transition-colors border-b border-gray-100",
                                img { class: "w-4 h-4 opacity-50", src: asset!("/assets/icons/inventory.svg") }
                                span { class: "flex-1 text-sm text-gray-700", "Products" }
                                span { class: "text-xs text-gray-500 font-mono", "{public_products} / {total_products}" }
                            }
                            Link { to: Route::AdminInventory {},
                                class: "flex items-center gap-3 px-4 py-2.5 hover:bg-gray-50 transition-colors border-b border-gray-100",
                                img { class: "w-4 h-4 opacity-50", src: asset!("/assets/icons/receipt.svg") }
                                span { class: "flex-1 text-sm text-gray-700", "Inventory" }
                                span { class: "text-xs text-gray-500 font-mono", "{total_stock} items" }
                            }
                            Link { to: Route::AdminUsers {},
                                class: "flex items-center gap-3 px-4 py-2.5 hover:bg-gray-50 transition-colors border-b border-gray-100",
                                img { class: "w-4 h-4 opacity-50", src: asset!("/assets/icons/people.svg") }
                                span { class: "flex-1 text-sm text-gray-700", "Users" }
                            }
                            Link { to: Route::AdminDiscounts {},
                                class: "flex items-center gap-3 px-4 py-2.5 hover:bg-gray-50 transition-colors border-b border-gray-100",
                                img { class: "w-4 h-4 opacity-50", src: asset!("/assets/icons/pricetags.svg") }
                                span { class: "flex-1 text-sm text-gray-700", "Discounts" }
                            }
                            Link { to: Route::AdminGroups {},
                                class: "flex items-center gap-3 px-4 py-2.5 hover:bg-gray-50 transition-colors border-b border-gray-100",
                                img { class: "w-4 h-4 opacity-50", src: asset!("/assets/icons/people-circle-outline.svg") }
                                span { class: "flex-1 text-sm text-gray-700", "Groups" }
                            }
                            Link { to: Route::AdminContent {},
                                class: "flex items-center gap-3 px-4 py-2.5 hover:bg-gray-50 transition-colors border-b border-gray-100",
                                img { class: "w-4 h-4 opacity-50", src: asset!("/assets/icons/images.svg") }
                                span { class: "flex-1 text-sm text-gray-700", "Content" }
                            }
                            Link { to: Route::AdminAnalytics {},
                                class: "flex items-center gap-3 px-4 py-2.5 hover:bg-gray-50 transition-colors",
                                img { class: "w-4 h-4 opacity-50", src: asset!("/assets/icons/bar-chart.svg") }
                                span { class: "flex-1 text-sm text-gray-700", "Analytics" }
                            }
                        }
                    }

                    // Low stock / stock OK
                    if !low_stock_items.is_empty() {
                        div {
                            class: "bg-white border border-orange-200",
                            div { class: "px-4 py-3 border-b border-orange-200 bg-orange-50",
                                p { class: "text-xs font-bold text-orange-800 uppercase tracking-wide", "⚠ Low Stock ({low_stock_count})" }
                            }
                            div {
                                for item in &low_stock_items {
                                    div {
                                        class: "flex items-center gap-2 px-4 py-2 border-b border-gray-100 last:border-b-0",
                                        span { class: "flex-1 text-xs font-medium text-gray-900 truncate", "{item.name}" }
                                        span { class: "text-xs text-orange-600 font-mono shrink-0",
                                            {
                                                let total: i32 = item.location_quantities.as_deref().unwrap_or(&[]).iter().map(|lq| lq.quantity).sum();
                                                format!("{} units", total)
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else if total_stock > 0 {
                        div {
                            class: "bg-white border border-green-200 px-4 py-3",
                            p { class: "text-xs font-medium text-green-700", "✓ All stock levels OK" }
                        }
                    }
                }
            }

            // ── Products row ─────────────────────────────────────────────────
            div {
                class: "bg-white border border-gray-200",
                div {
                    class: "px-4 py-3 border-b border-gray-200 flex items-center justify-between",
                    p { class: "text-xs font-bold text-gray-700 uppercase tracking-wide", "Products" }
                    div { class: "flex items-center gap-3 text-xs text-gray-500",
                        span { "{public_products} public" }
                        span { class: "text-gray-300", "·" }
                        span { "{total_products} total" }
                        span { class: "text-gray-300", "·" }
                        span { "{preorder_products} pre-order" }
                        span { class: "text-gray-300", "·" }
                        Link { to: Route::AdminProducts {}, class: "text-gray-600 hover:text-gray-900 transition-colors", "View all →" }
                    }
                }
                if recent_products.is_empty() {
                    p { class: "px-4 py-3 text-sm text-gray-400 italic", "Loading..." }
                } else {
                    div { class: "divide-y divide-gray-100",
                        for prod in &recent_products {
                            div {
                                class: "flex items-center gap-3 px-4 py-2.5",
                                div { class: "flex-1 min-w-0",
                                    p { class: "text-sm font-medium text-gray-900 truncate", "{prod.title}" }
                                    p { class: "text-xs text-gray-400", "{prod.product_form}" }
                                }
                                if prod.pre_order {
                                    span { class: "px-2 py-0.5 text-xs bg-blue-100 text-blue-800 rounded-full shrink-0", "Pre-order" }
                                }
                                if prod.back_order {
                                    span { class: "px-2 py-0.5 text-xs bg-gray-100 text-gray-600 rounded-full shrink-0", "Backorder" }
                                }
                                span {
                                    class: format!(
                                        "px-2 py-0.5 text-xs rounded-full shrink-0 {}",
                                        match prod.visibility {
                                            ProductVisibility::Public => "bg-green-100 text-green-800",
                                            ProductVisibility::Private => "bg-gray-100 text-gray-600",
                                            ProductVisibility::Unlisted => "bg-yellow-100 text-yellow-700",
                                        }
                                    ),
                                    "{prod.visibility}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn format_order_date(date: &chrono::NaiveDateTime) -> String {
    let local_date = Local::now().naive_local().date();
    let order_date = date.date();
    if order_date == local_date {
        "Today".to_string()
    } else if order_date == local_date - Duration::days(1) {
        "Yesterday".to_string()
    } else {
        date.format("%d %b").to_string()
    }
}

#[component]
fn RecentOrderRow(order: OrderInfo) -> Element {
    let status_str = if !order.backorder_reduces.is_empty() {
        "Backordered"
    } else if order.status == OrderStatus::Pending && order.prepared_at.is_some() {
        "Prepared"
    } else if order.status == OrderStatus::Fulfilled && order.tracking_url.is_none() {
        "Untracked"
    } else {
        match order.status {
            OrderStatus::Pending | OrderStatus::Processing => "Pending",
            OrderStatus::Paid => "Paid",
            OrderStatus::Fulfilled => "Fulfilled",
            OrderStatus::Cancelled => "Cancelled",
            OrderStatus::Refunded => "Refunded",
        }
    };

    let status_class = if !order.backorder_reduces.is_empty() {
        "px-2 py-0.5 bg-orange-100 text-orange-800 rounded-full text-xs shrink-0"
    } else {
        match order.status {
            OrderStatus::Pending | OrderStatus::Processing => {
                "px-2 py-0.5 bg-yellow-100 text-yellow-800 rounded-full text-xs shrink-0"
            }
            OrderStatus::Paid => {
                "px-2 py-0.5 bg-blue-100 text-blue-800 rounded-full text-xs shrink-0"
            }
            OrderStatus::Fulfilled => {
                "px-2 py-0.5 bg-green-100 text-green-800 rounded-full text-xs shrink-0"
            }
            OrderStatus::Cancelled => {
                "px-2 py-0.5 bg-gray-100 text-gray-600 rounded-full text-xs shrink-0"
            }
            OrderStatus::Refunded => {
                "px-2 py-0.5 bg-red-100 text-red-800 rounded-full text-xs shrink-0"
            }
        }
    };

    rsx! {
        div {
            class: "flex items-center gap-3 px-4 py-2.5 hover:bg-gray-50 border-b border-gray-100 last:border-b-0",
            div { class: "flex-1 min-w-0",
                p { class: "text-sm font-medium text-gray-900 font-mono", "#{order.ref_code}" }
                p { class: "text-xs text-gray-500 truncate", "{order.customer_email}" }
            }
            div { class: "text-right shrink-0",
                p { class: "text-sm font-medium text-gray-900 font-mono", "${order.total_amount_usd:.2}" }
                p { class: "text-xs text-gray-400", "{format_order_date(&order.created_at)}" }
            }
            span { class: "{status_class}", "{status_str}" }
        }
    }
}
