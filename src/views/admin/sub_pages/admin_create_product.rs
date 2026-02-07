#![allow(non_snake_case)] // Allow non-snake_case identifiers

use dioxus::prelude::*;

use strum::IntoEnumIterator;

use crate::Route;
use crate::backend::front_entities::*;
use crate::backend::server_functions::{
    CreateEditProductRequest, CreateEditProductVariantRequest, UploadResponse,
    admin_create_product, admin_upload_thumbnails,
};
use crate::components::*;

#[derive(Default, Clone, PartialEq, Debug)]
pub struct CreateProductVariant {
    pub name: String,
    pub primary_thumbnail_url: Option<String>,
    pub additional_thumbnail_urls: Option<Vec<String>>,
    pub price_base_standard_usd: f64,
    pub pbx_sku: String,
}

#[component]
pub fn AdminCreateProduct() -> Element {
    let mut title = use_signal(|| String::new());
    let mut subtitle = use_signal(|| String::new());
    let mut alternate_names: Signal<Vec<String>> = use_signal(|| vec![]);
    let mut handle = use_signal(|| String::new());

    let mut short_description = use_signal(|| String::new());
    let mut long_description = use_signal(|| String::new());

    let mut collections: Signal<Vec<Category>> = use_signal(|| vec![]);
    let mut product_form = use_signal(|| ProductForm::Solution);
    let mut visibility = use_signal(|| ProductVisibility::Private);

    let mut variants: Signal<Vec<CreateProductVariant>> = use_signal(|| vec![]);
    let mut uploading = use_signal(|| false);
    let mut creating = use_signal(|| false);

    let mut force_no_stock = use_signal(|| false);
    let mut purity_standard = use_signal(|| String::from("98.0"));

    // New fields
    let mut pre_order = use_signal(|| false);
    let mut pre_order_goal = use_signal(|| String::new());
    let mut brand = use_signal(|| String::new());
    let mut priority = use_signal(|| String::new());
    let mut back_order = use_signal(|| false);

    // Meta Info

    let mut physical_description = use_signal(|| String::new());
    let mut plabs_node_id = use_signal(|| String::new());
    let mut cas = use_signal(|| String::new());
    let mut iupac = use_signal(|| String::new());
    let mut mol_form = use_signal(|| String::new());
    let mut smiles = use_signal(|| String::new());
    let mut enable_render_if_smiles = use_signal(|| true);
    let mut pubchem_cid = use_signal(|| String::new());
    let mut analysis_url_qnmr = use_signal(|| String::new());
    let mut analysis_url_hplc = use_signal(|| String::new());
    let mut analysis_url_qh1 = use_signal(|| String::new());
    let mut weight = use_signal(|| 0f64);
    let mut dimensions_height = use_signal(|| 0f64);
    let mut dimensions_length = use_signal(|| 0f64);
    let mut dimensions_width = use_signal(|| 0f64);
    let mut phase: Signal<ProductPhase> = use_signal(|| ProductPhase::Blue);

    // Create notification signals
    let mut notification_message = use_signal(|| String::new());
    let mut notification_type = use_signal(|| String::new()); // "success" or "error"
    let mut show_notification = use_signal(|| false);

    let mut create_variant = move || {
        let mut current_variants = variants();
        current_variants.push(CreateProductVariant {
            name: String::new(),
            primary_thumbnail_url: None,
            additional_thumbnail_urls: None,
            price_base_standard_usd: 10f64,
            pbx_sku: String::from("9999"),
        });
        variants.set(current_variants);
    };

    let mut remove_variant = move |index: usize| {
        variants.with_mut(|v| {
            if index < v.len() {
                v.remove(index);
            }
        });
    };

    let handle_create_product = move |_| {
        spawn(async move {
            creating.set(true);

            // Validate required fields
            if title().trim().is_empty() {
                notification_message.set("Title is required".to_string());
                notification_type.set("error".to_string());
                show_notification.set(true);
                creating.set(false);
                return;
            }

            if handle().trim().is_empty() {
                notification_message.set("Handle is required".to_string());
                notification_type.set("error".to_string());
                show_notification.set(true);
                creating.set(false);
                return;
            }

            if variants().is_empty() {
                notification_message.set("At least one variant is required".to_string());
                notification_type.set("error".to_string());
                show_notification.set(true);
                creating.set(false);
                return;
            }

            // Validate variants
            for (i, variant) in variants().iter().enumerate() {
                if variant.name.trim().is_empty() {
                    notification_message.set(format!("Variant {} name is required", i + 1));
                    notification_type.set("error".to_string());
                    show_notification.set(true);
                    creating.set(false);
                    return;
                }
            }

            // Prepare request data
            let request = CreateEditProductRequest {
                id: None,
                title: title(),
                subtitle: if subtitle().trim().is_empty() {
                    None
                } else {
                    Some(subtitle())
                },
                handle: handle(),
                collections: collections(),
                short_description: if short_description().trim().is_empty() {
                    None
                } else {
                    Some(short_description())
                },
                long_description: if long_description().trim().is_empty() {
                    None
                } else {
                    Some(long_description())
                },
                alternate_names: alternate_names()
                    .into_iter()
                    .filter(|s| !s.trim().is_empty())
                    .collect(),
                product_form: product_form(),
                visibility: visibility(),
                force_no_stock: force_no_stock(),
                purity_standard: if purity_standard().trim().is_empty() {
                    None
                } else {
                    purity_standard().parse().ok()
                },
                pre_order: pre_order(),
                pre_order_goal: if pre_order_goal().trim().is_empty() {
                    None
                } else {
                    pre_order_goal().parse().ok()
                },
                brand: if brand().trim().is_empty() {
                    None
                } else {
                    Some(brand())
                },
                priority: if priority().trim().is_empty() {
                    None
                } else {
                    priority().parse().ok()
                },
                back_order: back_order(),
                physical_description: if physical_description().trim().is_empty() {
                    None
                } else {
                    Some(physical_description())
                },
                plabs_node_id: if plabs_node_id().trim().is_empty() {
                    None
                } else {
                    Some(plabs_node_id())
                },
                cas: if cas().trim().is_empty() {
                    None
                } else {
                    Some(cas())
                },
                iupac: if iupac().trim().is_empty() {
                    None
                } else {
                    Some(iupac())
                },
                mol_form: if mol_form().trim().is_empty() {
                    None
                } else {
                    Some(mol_form())
                },
                smiles: if smiles().trim().is_empty() {
                    None
                } else {
                    Some(smiles())
                },
                enable_render_if_smiles: enable_render_if_smiles(),
                pubchem_cid: if pubchem_cid().trim().is_empty() {
                    None
                } else {
                    Some(pubchem_cid())
                },
                analysis_url_qnmr: if analysis_url_qnmr().trim().is_empty() {
                    None
                } else {
                    Some(analysis_url_qnmr())
                },
                analysis_url_hplc: if analysis_url_hplc().trim().is_empty() {
                    None
                } else {
                    Some(analysis_url_hplc())
                },
                analysis_url_qh1: if analysis_url_qh1().trim().is_empty() {
                    None
                } else {
                    Some(analysis_url_qh1())
                },
                weight: if weight() == 0.0 {
                    None
                } else {
                    Some(weight())
                },
                dimensions_height: if dimensions_height() == 0.0 {
                    None
                } else {
                    Some(dimensions_height())
                },
                dimensions_length: if dimensions_length() == 0.0 {
                    None
                } else {
                    Some(dimensions_length())
                },
                dimensions_width: if dimensions_width() == 0.0 {
                    None
                } else {
                    Some(dimensions_width())
                },
                phase: phase(),
                variants: variants()
                    .into_iter()
                    .map(|v| CreateEditProductVariantRequest {
                        id: None,
                        name: v.name,
                        primary_thumbnail_url: v.primary_thumbnail_url,
                        additional_thumbnail_urls: v.additional_thumbnail_urls,
                        price_base_standard_usd: v.price_base_standard_usd,
                        pbx_sku: format!("PBX{}", v.pbx_sku),
                    })
                    .collect(),
                product_variant_stock_item_relations: None,
            };

            // Call server function
            match admin_create_product(request).await {
                Ok(response) => {
                    if response.success {
                        notification_message.set("Product created successfully!".to_string());
                        notification_type.set("success".to_string());
                        show_notification.set(true);

                        // Reset form
                        title.set(String::new());
                        subtitle.set(String::new());
                        handle.set(String::new());
                        short_description.set(String::new());
                        long_description.set(String::new());
                        alternate_names.set(vec![]);
                        variants.set(vec![]);
                        pre_order.set(false);
                        pre_order_goal.set(String::new());
                        brand.set(String::new());
                        priority.set(String::new());
                        back_order.set(false);
                        // Reset other fields as needed...
                    } else {
                        notification_message.set(response.message);
                        notification_type.set("error".to_string());
                        show_notification.set(true);
                    }
                }
                Err(e) => {
                    notification_message.set(format!("Error creating product: {}", e));
                    notification_type.set("error".to_string());
                    show_notification.set(true);
                }
            }

            creating.set(false);
        });
    };

    let handle_primary_thumbnail_upload = move |variant_index: usize| {
        move |evt: FormEvent| {
            let mut variants = variants.clone();
            let mut uploading = uploading.clone();

            spawn(async move {
                if let Some(file_engine) = evt.files() {
                    let files = file_engine.files();
                    if let Some(file_name) = files.get(0) {
                        uploading.set(true);

                        if let Some(file_data) = file_engine.read_file(file_name).await {
                            // Determine content type based on file extension
                            let content_type =
                                if file_name.ends_with(".jpg") || file_name.ends_with(".jpeg") {
                                    "image/jpeg"
                                } else if file_name.ends_with(".png") {
                                    "image/png"
                                } else if file_name.ends_with(".webp") {
                                    "image/webp"
                                } else if file_name.ends_with(".avif") {
                                    "image/avif"
                                } else {
                                    "image/jpeg" // default
                                };

                            match admin_upload_thumbnails(
                                file_data,
                                file_name.clone(),
                                content_type.to_string(),
                            )
                            .await
                            {
                                Ok(response) => {
                                    if response.success {
                                        variants.with_mut(|v| {
                                            if let Some(variant) = v.get_mut(variant_index) {
                                                variant.primary_thumbnail_url = response.url;
                                            }
                                        });
                                    } else {
                                        // Handle error - you might want to show a toast or error message
                                        println!("Upload failed: {}", response.message);
                                    }
                                }
                                Err(e) => {
                                    println!("Upload error: {}", e);
                                }
                            }
                        }

                        uploading.set(false);
                    }
                }
            });
        }
    };

    let handle_additional_thumbnail_upload = move |variant_index: usize| {
        move |evt: FormEvent| {
            let mut variants = variants.clone();
            let mut uploading = uploading.clone();

            spawn(async move {
                if let Some(file_engine) = evt.files() {
                    let files = file_engine.files();
                    if let Some(file_name) = files.get(0) {
                        uploading.set(true);

                        if let Some(file_data) = file_engine.read_file(file_name).await {
                            let content_type =
                                if file_name.ends_with(".jpg") || file_name.ends_with(".jpeg") {
                                    "image/jpeg"
                                } else if file_name.ends_with(".png") {
                                    "image/png"
                                } else if file_name.ends_with(".webp") {
                                    "image/webp"
                                } else if file_name.ends_with(".avif") {
                                    "image/avif"
                                } else {
                                    "image/jpeg"
                                };

                            match admin_upload_thumbnails(
                                file_data,
                                file_name.clone(),
                                content_type.to_string(),
                            )
                            .await
                            {
                                Ok(response) => {
                                    if response.success {
                                        variants.with_mut(|v| {
                                            if let Some(variant) = v.get_mut(variant_index) {
                                                if variant.additional_thumbnail_urls.is_none() {
                                                    variant.additional_thumbnail_urls =
                                                        Some(vec![]);
                                                }
                                                if let Some(ref mut urls) =
                                                    variant.additional_thumbnail_urls
                                                {
                                                    if let Some(url) = response.url {
                                                        urls.push(url);
                                                    }
                                                }
                                            }
                                        });
                                    } else {
                                        println!("Upload failed: {}", response.message);
                                    }
                                }
                                Err(e) => {
                                    println!("Upload error: {}", e);
                                }
                            }
                        }

                        uploading.set(false);
                    }
                }
            });
        }
    };

    rsx! {
        // Notification
        if show_notification() {
            div {
                class: format!("fixed top-4 right-4 z-50 p-4 rounded-md shadow-lg text-white {}",
                    if notification_type() == "success" { "bg-green-500" } else { "bg-red-500" }
                ),
                div {
                    class: "flex justify-between items-center",
                    span { "{notification_message()}" }
                    button {
                        class: "ml-4 text-white hover:text-gray-200",
                        onclick: move |_| show_notification.set(false),
                        "×"
                    }
                }
            }
        }

        div {
            class: "border rounded-md border-gray-200 w-full mb-4 flex justify-between items-center p-2 pl-4",
            div {
                class: "text-lg font-medium",
                "Create New Product"
            }
            button {
                class: format!("text-sm px-3 py-2 text-white rounded transition-colors {}",
                    if creating() { "bg-gray-500 cursor-not-allowed" } else { "bg-gray-900 hover:bg-gray-800" }
                ),
                disabled: creating(),
                onclick: handle_create_product,
                if creating() {
                    "Creating..."
                } else {
                    "Create"
                }
            }
        }

        div {
            class: "flex flex-col md:flex-row w-full gap-2",
            div {
                class: "flex w-full flex-col gap-2",
                div {
                    class: "flex-grow bg-white border rounded-md border-gray-200 p-4 min-h-36",
                    div {
                        class: "flex gap-4 w-full",

                        div {
                            class: "w-full",
                            CTextBox {
                                label: "Title",
                                value: "{title}",
                                optional: false,
                                oninput: move |event: FormEvent| title.set(event.value())
                            }
                        },

                        div {
                            class: "w-full",
                            CTextBox {
                                label: "Subtitle",
                                placeholder: "5mg/mL",
                                value: "{subtitle}",
                                optional: true,
                                oninput: move |event: FormEvent| subtitle.set(event.value())
                            }
                        },

                        div {
                            class: "w-full",
                            CTextBox {
                                label: "Handle",
                                placeholder: "product-name",
                                value: "{handle}",
                                optional: false,
                                prefix: "/",
                                oninput: move |event: FormEvent| {
                                    let filtered_value: String = event.value()
                                        .chars()
                                        .filter(|&c| c.is_ascii_alphanumeric() || c == '-')
                                        .collect();
                                    handle.set(filtered_value);
                                }
                            }
                        },
                    }
                    br {},
                    CTextArea {
                        label: "Short Description (md)",
                        placeholder: "About this product...",
                        value: "{short_description}",
                        oninput: move |event: FormEvent| short_description.set(event.value())
                    },
                },
                div {
                    class: "flex-grow bg-white border rounded-md border-gray-200 min-h-36",
                    div {
                        class: "flex justify-between border-b border-gray-300",
                        h2 {
                            class: "text-lg pl-4 pt-3.5 pb-2",
                            "Variants"
                        },
                        div {
                            button {
                                class: "text-sm bg-gray-900 px-3 py-2 text-white rounded hover:bg-gray-800 transition-colors m-2",
                                onclick: move |_| create_variant(),
                                "Add Variant"
                            }
                        }
                    },

                    if variants.read().len() == 0 {
                        div {
                            class: "text-gray-500 w-full py-8 text-center text-sm",
                            "No variants created yet"
                        }
                    } else {
                        div {
                            class: "",
                            for (index, variant) in variants.read().iter().enumerate() {
                                div {
                                    class: "p-4 border-t border-gray-300 first:border-t-0",

                                    div {
                                        class: "flex justify-between items-center mb-4",
                                        h3 {
                                            class: "text-xs text-uppercase text-gray-700 font-bold",
                                            "VARIANT {index + 1}"
                                        },
                                        button {
                                            class: "text-red-600 hover:text-red-800 text-sm",
                                            onclick: move |_| remove_variant(index),
                                            "Remove"
                                        }
                                    },

                                    div {
                                        class: "flex gap-4",

                                        // Main thumbnail section (30% width)
                                        div {
                                            class: "w-[30%] min-w-[120px]",
                                            div {
                                                class: "mb-2",
                                                label {
                                                    class: "block text-sm font-medium text-gray-700 mb-1",
                                                    "Main Thumbnail"
                                                }
                                            },
                                            div {
                                                class: "aspect-square w-full border-2 border-dashed border-gray-300 rounded-lg hover:border-gray-400 transition-colors cursor-pointer bg-gray-50 hover:bg-gray-100 flex flex-col items-center justify-center relative overflow-hidden",

                                                if let Some(url) = &variant.primary_thumbnail_url {
                                                    // Display uploaded image
                                                    img {
                                                        src: "{url}",
                                                        class: "w-full h-full object-cover rounded-lg",
                                                        alt: "Uploaded thumbnail"
                                                    }
                                                } else {
                                                    // Upload prompt
                                                    div {
                                                        class: "text-center p-4",
                                                        if uploading() {
                                                            div {
                                                                class: "animate-spin rounded-full h-8 w-8 border-b-2 border-gray-900 mx-auto mb-2"
                                                            }
                                                            span {
                                                                class: "text-sm text-gray-600",
                                                                "Uploading..."
                                                            }
                                                        } else {
                                                            svg {
                                                                class: "mx-auto h-8 w-8 text-gray-400 mb-2",
                                                                stroke: "currentColor",
                                                                fill: "none",
                                                                view_box: "0 0 48 48",
                                                                path {
                                                                    d: "M28 8H12a4 4 0 00-4 4v20m32-12v8m0 0v8a4 4 0 01-4 4H12a4 4 0 01-4-4v-4m32-4l-3.172-3.172a4 4 0 00-5.656 0L28 28M8 32l9.172-9.172a4 4 0 015.656 0L28 28m0 0l4 4m4-24h8m-4-4v8m-12 4h.02",
                                                                    stroke_width: "2",
                                                                    stroke_linecap: "round",
                                                                    stroke_linejoin: "round",
                                                                }
                                                            }
                                                            span {
                                                                class: "text-sm text-gray-600 font-medium",
                                                                "Drop image"
                                                            }
                                                            p {
                                                                class: "text-xs text-gray-500 mt-1",
                                                                "or click to browse"
                                                            }
                                                        }
                                                    }
                                                }

                                                // Hidden file input
                                                input {
                                                    r#type: "file",
                                                    accept: "image/*",
                                                    class: "absolute inset-0 w-full h-full opacity-0 cursor-pointer",
                                                    onchange: handle_primary_thumbnail_upload(index),
                                                    disabled: uploading()
                                                }
                                            }
                                        },

                                        // Right side content (70% width)
                                        div {
                                            class: "flex-1 flex flex-col gap-4",

                                            // Name and Price inputs
                                            div {
                                                class: "grid grid-cols-1 md:grid-cols-2 gap-4",

                                                div {
                                                    CTextBox {
                                                        label: "Variant Name",
                                                        value: variant.name.clone(),
                                                        placeholder: "e.g., 15mL, 30mL, 60mL",
                                                        optional: false,
                                                        oninput: move |event: FormEvent| {
                                                            variants.with_mut(|v| {
                                                                if let Some(variant) = v.get_mut(index) {
                                                                    variant.name = event.value();
                                                                }
                                                            });
                                                        }
                                                    }
                                                },

                                                div {
                                                    CTextBox {
                                                        label: "Base Price (USD)",
                                                        value: format!("{:.2}", variant.price_base_standard_usd),
                                                        placeholder: "10.00",
                                                        optional: false,
                                                        prefix: "$",
                                                        is_number: true,
                                                        step: 1f64,
                                                        oninput: move |event: FormEvent| {
                                                            if let Ok(price) = event.value().parse::<f64>() {
                                                                variants.with_mut(|v| {
                                                                    if let Some(variant) = v.get_mut(index) {
                                                                        variant.price_base_standard_usd = price;
                                                                    }
                                                                });
                                                            }
                                                        }
                                                    }
                                                }
                                            },

                                            div {
                                                CTextBox {
                                                    label: "SKU",
                                                    value: variant.pbx_sku.clone(),
                                                    placeholder: "9999",
                                                    optional: false,
                                                    prefix: "PBX",
                                                    oninput: move |event: FormEvent| {
                                                        variants.with_mut(|v| {
                                                            if let Some(variant) = v.get_mut(index) {
                                                                variant.pbx_sku = event.value();
                                                            }
                                                        });
                                                    }
                                                }
                                            }

                                            // Additional thumbnails section
                                            div {
                                                div {
                                                    class: "mb-2",
                                                    label {
                                                        class: "block text-sm font-medium text-gray-700",
                                                        "Additional Thumbnails"
                                                    }
                                                },
                                                div {
                                                    class: "flex gap-2 overflow-x-auto pb-2",

                                                    // Existing additional thumbnails
                                                    if let Some(additional_urls) = &variant.additional_thumbnail_urls {
                                                        for (thumb_index, url) in additional_urls.iter().enumerate() {
                                                            div {
                                                                class: "relative flex-shrink-0 w-24 h-24 border-2 border-gray-300 rounded-lg overflow-hidden",
                                                                img {
                                                                    src: "{url}",
                                                                    class: "w-full h-full object-cover",
                                                                    alt: "Additional thumbnail {thumb_index + 1}"
                                                                }
                                                                button {
                                                                    class: "absolute top-0 right-0 bg-red-500 text-white rounded-full w-5 h-5 flex items-center justify-center text-xs hover:bg-red-600 transform translate-x-1 -translate-y-1",
                                                                    onclick: move |_| {
                                                                        variants.with_mut(|v| {
                                                                            if let Some(variant) = v.get_mut(index) {
                                                                                if let Some(ref mut urls) = variant.additional_thumbnail_urls {
                                                                                    if thumb_index < urls.len() {
                                                                                        urls.remove(thumb_index);
                                                                                    }
                                                                                }
                                                                            }
                                                                        });
                                                                    },
                                                                    "×"
                                                                }
                                                            }
                                                        }
                                                    }

                                                    // Add new thumbnail button
                                                    div {
                                                        class: "relative flex-shrink-0 w-24 h-24 border-2 border-dashed border-gray-300 rounded-lg hover:border-gray-400 transition-colors cursor-pointer bg-gray-50 hover:bg-gray-100 flex items-center justify-center",

                                                        if uploading() {
                                                            div {
                                                                class: "animate-spin rounded-full h-4 w-4 border-b-2 border-gray-600"
                                                            }
                                                        } else {
                                                            svg {
                                                                class: "w-6 h-6 text-gray-400",
                                                                stroke: "currentColor",
                                                                fill: "none",
                                                                view_box: "0 0 24 24",
                                                                path {
                                                                    d: "M12 4v16m8-8H4",
                                                                    stroke_width: "2",
                                                                    stroke_linecap: "round",
                                                                    stroke_linejoin: "round",
                                                                }
                                                            }
                                                        }

                                                        input {
                                                            r#type: "file",
                                                            accept: "image/*",
                                                            class: "absolute inset-0 w-full h-full opacity-0 cursor-pointer",
                                                            onchange: handle_additional_thumbnail_upload(index),
                                                            disabled: uploading()
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
                },
                div {
                    class: "flex-grow bg-white border p-4 rounded-md border-gray-200 min-h-36",
                    CTextArea {
                        label: "Main Description (md)",
                        placeholder: "More detailed description...",
                        value: "{long_description}",
                        oninput: move |event: FormEvent| long_description.set(event.value())
                    },
                }
            }
            div {
                class: "md:w-[38%] w-full min-w-0",
                div {
                    class: "bg-white border flex flex-col gap-4 rounded-md border-gray-200 p-4 min-h-36",
                    div {

                        div {
                            class: "text-xs font-medium text-gray-700 pb-1",
                            "Collections"
                        },
                        div {
                            class: "flex flex-wrap gap-2 mb-4",
                            for collection in Category::iter() {
                                div {
                                    class: if collections().contains(&collection) {
                                        "border-2 border-sky-500 gap-x-1 bg-gray-100 px-2 py-1 cursor-pointer rounded-md"
                                    } else {
                                        "border border-gray-200 gap-x-1 bg-gray-100 px-2 py-1 cursor-pointer rounded-md"
                                    },
                                    onclick: move |_| {
                                        let mut current_collections = collections();
                                        if current_collections.contains(&collection) {
                                            current_collections.retain(|c| c != &collection);
                                        } else {
                                            current_collections.push(collection);
                                        }
                                        collections.set(current_collections);
                                    },
                                    span {
                                        class: "text-sm",
                                        "{collection.to_string()}"
                                    }
                                }
                            }
                        }

                        // Label (shown above alternate names)
                        div {
                            class: "text-xs font-medium text-gray-700 pb-1",
                            "Alternate Names"
                        },

                        // Container for all textboxes with vertical gap
                        div {
                            class: "flex flex-col gap-2",

                            // Always ensure we have at least one entry to display
                            {
                                let display_names = if alternate_names.read().is_empty() {
                                    vec![String::new()]
                                } else {
                                    alternate_names.read().clone()
                                };

                                rsx! {
                                    for (index, alt_name) in display_names.iter().enumerate() {
                                        div {
                                            class: "flex flex-row items-center gap-2",
                                            div {
                                                class: "flex-1",
                                                CTextBox {
                                                    placeholder: if index == 0 && alt_name.is_empty() { "Enter alternate name..." } else { "" },
                                                    value: alt_name.clone(),
                                                    prefix: "{index + 1}",
                                                    optional: true,
                                                    oninput: move |event: FormEvent| {
                                                        let mut new_names = alternate_names.read().clone();

                                                        // If vector is empty and this is the first textbox, initialize it
                                                        if new_names.is_empty() {
                                                            new_names.push(event.value());
                                                        } else if index < new_names.len() {
                                                            new_names[index] = event.value();
                                                        }

                                                        alternate_names.set(new_names);
                                                    }
                                                }
                                            },

                                            // Only show + button for the first textbox
                                            if index == 0 {
                                                {
                                                    let first_has_content = !alternate_names.read().get(0).unwrap_or(&String::new()).trim().is_empty();

                                                    rsx! {
                                                        button {
                                                            class: if first_has_content {
                                                                "text-blue-500 hover:text-blue-700 text-lg font-bold w-8 h-8 flex items-center justify-center rounded hover:bg-blue-50 border border-blue-300 hover:border-blue-500 cursor-pointer"
                                                            } else {
                                                                "text-gray-300 text-lg font-bold w-8 h-8 flex items-center justify-center rounded border border-gray-200 cursor-not-allowed"
                                                            },
                                                            disabled: !first_has_content,
                                                            onclick: move |_| {
                                                                // Only execute if first textbox has content
                                                                if !alternate_names.read().get(0).unwrap_or(&String::new()).trim().is_empty() {
                                                                    let mut new_names = alternate_names.read().clone();
                                                                    new_names.push(String::new());
                                                                    alternate_names.set(new_names);
                                                                }
                                                            },
                                                            "+"
                                                        }
                                                    }
                                                }
                                            } else {
                                                // Add empty space for alignment on other textboxes
                                                div {
                                                    class: "w-8 h-8"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    div {
                        CSelectGroup {
                            label: "Product Form",
                            optional: false,
                            oninput: move |event: FormEvent| {
                                if let Ok(pform) = event.value().parse::<ProductForm>() {
                                    product_form.set(pform);
                                }
                            },
                            for product_form_type in ProductForm::iter() {
                                CSelectItem {
                                    selected: if product_form_type == ProductForm::Solution { true } else { false },
                                    key: "{product_form_type:?}", // Add a key for each item
                                    "{product_form_type.to_string()}"
                                }
                            }
                        },
                    }
                    div {
                        class: "",
                        CSelectGroup {
                            label: "Visibility",
                            optional: false,
                            oninput: move |event: FormEvent| {
                                if let Ok(vis) = event.value().parse::<ProductVisibility>() {
                                    visibility.set(vis);
                                }
                            },
                            for product_vis_type in ProductVisibility::iter() {
                                CSelectItem {
                                    value: "{product_vis_type}",
                                    selected: if product_vis_type == ProductVisibility::Private { true } else { false },
                                    key: "{product_vis_type:?}",
                                    "{product_vis_type.to_string()}"
                                }
                            }
                        }
                    }
                    div {
                        class: "w-full",
                        CTextBox {
                            label: "Purity Standard",
                            placeholder: "98.0",
                            prefix: "%",
                            is_number: true,
                            step: 0.1f64,
                            value: "{purity_standard}",
                            optional: true,
                            oninput: move |event: FormEvent| purity_standard.set(event.value())
                        }
                    },
                    div {
                        class: "w-full flex justify-between px-3.5 py-2 border border-gray-200 rounded-md",
                        p {
                            class: "text-sm text-gray-700 pt-[2px]",
                            "Force No Stock"
                        }
                        CToggle {
                            checked: force_no_stock(),
                            onclick: move |_| force_no_stock.toggle()
                        }
                    }
                    // New fields below Force No Stock
                    div {
                        class: "w-full flex justify-between px-3.5 py-2 border border-gray-200 rounded-md",
                        p {
                            class: "text-sm text-gray-700 pt-[2px]",
                            "Pre Order"
                        }
                        CToggle {
                            checked: pre_order(),
                            onclick: move |_| pre_order.toggle()
                        }
                    }
                    if pre_order() {
                        div {
                            class: "w-full",
                            CTextBox {
                                label: "Pre Order Goal",
                                placeholder: "1000.00",
                                prefix: "$",
                                is_number: true,
                                step: 0.01f64,
                                value: "{pre_order_goal}",
                                optional: true,
                                oninput: move |event: FormEvent| pre_order_goal.set(event.value())
                            }
                        }
                    }
                    div {
                        class: "w-full",
                        CTextBox {
                            label: "Priority",
                            placeholder: "1",
                            is_number: true,
                            step: 1f64,
                            value: "{priority}",
                            optional: true,
                            oninput: move |event: FormEvent| priority.set(event.value())
                        }
                    }
                    div {
                        class: "w-full",
                        CTextBox {
                            label: "Brand",
                            placeholder: "Brand name",
                            value: "{brand}",
                            optional: true,
                            oninput: move |event: FormEvent| brand.set(event.value())
                        }
                    }
                    div {
                        class: "w-full flex justify-between px-3.5 py-2 border border-gray-200 rounded-md",
                        p {
                            class: "text-sm text-gray-700 pt-[2px]",
                            "Back Order"
                        }
                        CToggle {
                            checked: back_order(),
                            onclick: move |_| back_order.toggle()
                        }
                    }
                }
                div {
                    class: "mt-2 bg-white border rounded-md border-gray-200 p-4 min-h-36",
                    div {
                        class: "w-full flex flex-col gap-4",
                        div {
                            class: "w-full",
                            CTextBox {
                                label: "Physical Description",
                                placeholder: "Liquid, solved in...",
                                value: "{physical_description}",
                                optional: true,
                                oninput: move |event: FormEvent| physical_description.set(event.value())
                            }
                        },
                        div {
                            class: "w-full",
                            CTextBox {
                                label: "Penchant Labs Node ID",
                                placeholder: "c-id",
                                value: "{plabs_node_id}",
                                optional: true,
                                oninput: move |event: FormEvent| plabs_node_id.set(event.value())
                            }
                        },
                        div {
                            class: "w-full",
                            CTextBox {
                                label: "Pubchem CID",
                                placeholder: "5427...",
                                value: "{pubchem_cid}",
                                optional: true,
                                oninput: move |event: FormEvent| pubchem_cid.set(event.value())
                            }
                        },
                        div {
                            class: "w-full",
                            CTextBox {
                                label: "CAS Code",
                                placeholder: "1358...",
                                value: "{cas}",
                                optional: true,
                                oninput: move |event: FormEvent| cas.set(event.value())
                            }
                        },
                        div {
                            class: "w-full",
                            CTextBox {
                                label: "IUPAC Code",
                                placeholder: "9-(4-cyclo",
                                value: "{iupac}",
                                optional: true,
                                oninput: move |event: FormEvent| iupac.set(event.value())
                            }
                        },
                        div {
                            class: "w-full",
                            CTextBox {
                                label: "Molecular Formula",
                                placeholder: "C19H2...",
                                value: "{mol_form}",
                                optional: true,
                                oninput: move |event: FormEvent| mol_form.set(event.value())
                            }
                        },
                        div {
                            class: "w-full",
                            CTextBox {
                                label: "SMILES Code",
                                placeholder: "C(CC...",
                                value: "{smiles}",
                                optional: true,
                                oninput: move |event: FormEvent| smiles.set(event.value())
                            }
                        },
                        div {
                            class: "w-full flex justify-between px-3.5 py-2 border border-gray-200 rounded-md",
                            p {
                                class: "text-sm text-gray-700 pt-[2px]",
                                "Enable Smiles Render"
                            }
                            CToggle {
                                checked: enable_render_if_smiles(),
                                onclick: move |_| enable_render_if_smiles.toggle()
                            }
                        },
                    }
                }
                div {
                    class: "mt-2 bg-white border rounded-md border-gray-200 p-4 min-h-36",
                    div {
                        class: "w-full flex flex-col gap-4",
                        div {
                            class: "w-full",
                            CTextBox {
                                label: "Analysis Link (QNMR)",
                                placeholder: "Proton folder link",
                                value: "{analysis_url_qnmr}",
                                optional: true,
                                oninput: move |event: FormEvent| analysis_url_qnmr.set(event.value())
                            }
                        },
                        div {
                            class: "w-full",
                            CTextBox {
                                label: "Analysis Link (HPLC)",
                                placeholder: "Proton folder link",
                                value: "{analysis_url_hplc}",
                                optional: true,
                                oninput: move |event: FormEvent| analysis_url_hplc.set(event.value())
                            }
                        },
                        div {
                            class: "w-full",
                            CTextBox {
                                label: "Analysis Link (Q-H1)",
                                placeholder: "Proton folder link",
                                value: "{analysis_url_qh1}",
                                optional: true,
                                oninput: move |event: FormEvent| analysis_url_qh1.set(event.value())
                            }
                        },
                        div {
                            CSelectGroup {
                                label: "Product Phase",
                                optional: false,
                                oninput: move |event: FormEvent| {
                                    if let Ok(pphase) = event.value().parse::<ProductPhase>() {
                                        phase.set(pphase);
                                    }
                                },
                                for product_phase_type in ProductPhase::iter() {
                                    CSelectItem {
                                        selected: if product_phase_type == ProductPhase::Blue { true } else { false },
                                        key: "{product_phase_type:?}", // Add a key for each item
                                        "{product_phase_type.to_string()}"
                                    }
                                }
                            },
                        },
                        div {
                            CTextBox {
                                label: "Weight",
                                placeholder: "",
                                prefix: "g",
                                is_number: true,
                                step: 1f64,
                                value: "{weight}",
                                optional: true,
                                oninput: move |event: FormEvent| {
                                    if let Ok(w) = event.value().parse::<f64>() {
                                        weight.set(w);
                                    }
                                }
                            },
                        },
                        div {
                            class: "flex justify-between gap-4",
                            CTextBox {
                                label: "Height",
                                placeholder: "",
                                prefix: "mm",
                                is_number: true,
                                step: 1f64,
                                value: "{dimensions_height}",
                                optional: true,
                                oninput: move |event: FormEvent| {
                                    if let Ok(height) = event.value().parse::<f64>() {
                                        dimensions_height.set(height);
                                    }
                                }
                            },
                            CTextBox {
                                label: "Length",
                                placeholder: "",
                                prefix: "mm",
                                is_number: true,
                                step: 1f64,
                                value: "{dimensions_length}",
                                optional: true,
                                oninput: move |event: FormEvent| {
                                    if let Ok(length) = event.value().parse::<f64>() {
                                        dimensions_length.set(length);
                                    }
                                }
                            },
                            CTextBox {
                                label: "Width",
                                placeholder: "",
                                prefix: "mm",
                                is_number: true,
                                step: 1f64,
                                value: "{dimensions_width}",
                                optional: true,
                                oninput: move |event: FormEvent| {
                                    if let Ok(width) = event.value().parse::<f64>() {
                                        dimensions_width.set(width);
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
