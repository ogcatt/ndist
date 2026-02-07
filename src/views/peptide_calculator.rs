use crate::Route;
use crate::components::{CSelectGroup, CSelectItem, CTextBox, SelectPlaceholder};
use dioxus::prelude::*;
use dioxus_i18n::t;

#[component]
pub fn PeptideCalculator() -> Element {
    let mut vol = use_signal(|| "0.5".to_string());
    let mut vial_mg = use_signal(|| String::new());
    let mut temp_vial_mg = use_signal(|| String::new());
    let mut vial_mg_other = use_signal(|| false);
    let mut bac_ml = use_signal(|| String::new());
    let mut sample_mg = use_signal(|| String::new());
    let mut temp_sample_mg = use_signal(|| String::new());
    let mut sample_other = use_signal(|| false);

    // Calculate the result
    let calculate_result = move || -> Option<f64> {
        let vial_mg_val: f64 = vial_mg().parse().ok()?;
        let bac_ml_val: f64 = bac_ml().parse().ok()?;
        let sample_mg_val: f64 = sample_mg().parse().ok()?;
        let vol_val: f64 = vol().parse().ok()?;

        if vial_mg_val > 0.0 && bac_ml_val > 0.0 && sample_mg_val > 0.0 && vol_val > 0.0 {
            let result =
                ((sample_mg_val / (vial_mg_val / bac_ml_val)) * 100.0 * 10.0).round() / 10.0;
            Some(result)
        } else {
            None
        }
    };

    // Calculate ruler percentage
    let calculate_ruler_percentage = move || -> f64 {
        if let Some(result) = calculate_result() {
            let vol_val: f64 = vol().parse().unwrap_or(1.0);
            let percentage = (result / vol_val).min(100.0);
            percentage
        } else {
            0.0
        }
    };

    // Get ruler image path based on volume
    let get_ruler_image = move || {
        match vol().as_str() {
            "0.3" => asset!("/assets/images/ruler-0.3.webp"),
            "0.5" => asset!("/assets/images/ruler-0.5.webp"),
            "1" => asset!("/assets/images/ruler-1.webp"),
            //"2" => asset!("/assets/images/ruler-2.webp"),
            _ => asset!("/assets/images/ruler-0.5.webp"), // default fallback
        }
    };

    rsx! {
        document::Title { { format!("{} - {}", t!("brand"), t!("peptide-calculator")) } }

        div {
            class: "py-12 flex justify-center",
            div {
                class: "max-w-[1000px] w-full px-4",
                h2 {
                    class: "mb-6 text-2xl",
                    { t!("peptide-calculator-title") }
                }

                // Syringe volume selection
                div {
                    class: "max-w-80 mb-4",
                    CSelectGroup {
                        large: true,
                        label: t!("syringe-volume-label"),
                        oninput: move |e: FormEvent| {
                            vol.set(e.value());
                        },
                        SelectPlaceholder { { t!("select-ml-volume") } }
                        CSelectItem {
                            selected: if vol() == "0.3" { true } else { false },
                            value: "0.3",
                            { t!("volume-03ml") }
                        }
                        CSelectItem {
                            selected: if vol() == "0.5" { true } else { false },
                            value: "0.5",
                            { t!("volume-05ml") }
                        }
                        CSelectItem {
                            selected: if vol() == "1" { true } else { false },
                            value: "1",
                            { t!("volume-1ml") }
                        }
                        //CSelectItem {
                        //    selected: if vol() == "2" { true } else { false },
                        //    value: "2",
                        //    { t!("volume-2ml") }
                        //}
                    }
                }

                // Vial mg selection
                div {
                    class: "max-w-80 mb-4",
                    CSelectGroup {
                        large: true,
                        label: t!("vial-peptide-amount-label"),
                        oninput: move |e: FormEvent| {
                            let value = e.value();
                            temp_vial_mg.set(value.clone());
                            if value == "other" {
                                vial_mg_other.set(true);
                            } else {
                                vial_mg_other.set(false);
                                vial_mg.set(value);
                            }
                        },
                        SelectPlaceholder { { t!("select-vial-mg") } }
                        CSelectItem {
                            selected: if temp_vial_mg() == "1" { true } else { false },
                            value: "1",
                            { t!("vial-1mg") }
                        }
                        CSelectItem {
                            selected: if temp_vial_mg() == "2" { true } else { false },
                            value: "2",
                            { t!("vial-2mg") }
                        }
                        CSelectItem {
                            selected: if temp_vial_mg() == "5" { true } else { false },
                            value: "5",
                            { t!("vial-5mg") }
                        }
                        CSelectItem {
                            selected: if temp_vial_mg() == "10" { true } else { false },
                            value: "10",
                            { t!("vial-10mg") }
                        }
                        CSelectItem {
                            selected: if temp_vial_mg() == "15" { true } else { false },
                            value: "15",
                            { t!("vial-15mg") }
                        }
                        CSelectItem {
                            selected: if temp_vial_mg() == "20" { true } else { false },
                            value: "20",
                            { t!("vial-20mg") }
                        }
                        CSelectItem {
                            selected: if temp_vial_mg() == "40" { true } else { false },
                            value: "40",
                            { t!("vial-40mg") }
                        }
                        CSelectItem {
                            selected: if temp_vial_mg() == "other" { true } else { false },
                            value: "other",
                            { t!("other") }
                        }
                    }
                }

                if vial_mg_other() {
                    div {
                        class: "mt-2 mb-4 max-w-80",
                        CTextBox {
                            large: true,
                            value: vial_mg(),
                            label: t!("vial-mg-input-label"),
                            is_number: true,
                            oninput: move |e: FormEvent| {
                                vial_mg.set(e.value());
                            }
                        }
                    }
                }

                // Bacteriostatic water selection
                div {
                    class: "max-w-80 mb-4",
                    CSelectGroup {
                        large: true,
                        label: t!("bac-water-amount-label"),
                        oninput: move |e: FormEvent| {
                            bac_ml.set(e.value());
                        },
                        SelectPlaceholder { { t!("select-bac-ml") } }
                        CSelectItem {
                            selected: if bac_ml() == "1" { true } else { false },
                            value: "1",
                            { t!("bac-1ml") }
                        }
                        CSelectItem {
                            selected: if bac_ml() == "2" { true } else { false },
                            value: "2",
                            { t!("bac-2ml") }
                        }
                        CSelectItem {
                            selected: if bac_ml() == "3" { true } else { false },
                            value: "3",
                            { t!("bac-3ml") }
                        }
                        CSelectItem {
                            selected: if bac_ml() == "4" { true } else { false },
                            value: "4",
                            { t!("bac-4ml") }
                        }
                        CSelectItem {
                            selected: if bac_ml() == "5" { true } else { false },
                            value: "5",
                            { t!("bac-5ml") }
                        }
                        CSelectItem {
                            selected: if bac_ml() == "10" { true } else { false },
                            value: "10",
                            { t!("bac-10ml") }
                        }
                    }
                }

                // Sample mg selection
                div {
                    class: "max-w-80 mb-4",

                    CSelectGroup {
                        large: true,
                        label: t!("sample-peptide-amount-label"),
                        oninput: move |e: FormEvent| {
                            let value = e.value();
                            temp_sample_mg.set(value.clone());
                            if value == "other" {
                                sample_other.set(true);
                            } else {
                                sample_other.set(false);
                                sample_mg.set(value);
                            }
                        },
                        SelectPlaceholder { { t!("select-compound-mg") } }
                        CSelectItem {
                            selected: if temp_sample_mg() == "0.1" { true } else { false },
                            value: "0.1",
                            { t!("sample-100mcg") }
                        }
                        CSelectItem {
                            selected: if temp_sample_mg() == "0.2" { true } else { false },
                            value: "0.2",
                            { t!("sample-200mcg") }
                        }
                        CSelectItem {
                            selected: if temp_sample_mg() == "0.25" { true } else { false },
                            value: "0.25",
                            { t!("sample-250mcg") }
                        }
                        CSelectItem {
                            selected: if temp_sample_mg() == "0.5" { true } else { false },
                            value: "0.5",
                            { t!("sample-500mcg") }
                        }
                        CSelectItem {
                            selected: if temp_sample_mg() == "1" { true } else { false },
                            value: "1",
                            { t!("sample-1mg") }
                        }
                        CSelectItem {
                            selected: if temp_sample_mg() == "3" { true } else { false },
                            value: "3",
                            { t!("sample-3mg") }
                        }
                        CSelectItem {
                            selected: if temp_sample_mg() == "5" { true } else { false },
                            value: "5",
                            { t!("sample-5mg") }
                        }
                        CSelectItem {
                            selected: if temp_sample_mg() == "10" { true } else { false },
                            value: "10",
                            { t!("sample-10mg") }
                        }
                        CSelectItem {
                            selected: if temp_sample_mg() == "other" { true } else { false },
                            value: "other",
                            { t!("other") }
                        }
                    }
                }

                if sample_other() {
                    div {
                        class: "mt-2 max-w-80",
                        CTextBox {
                            large: true,
                            value: sample_mg(),
                            label: t!("compound-mg-input-label"),
                            is_number: true,
                            oninput: move |e: FormEvent| {
                                sample_mg.set(e.value());
                            }
                        }
                    }
                }

                // Ruler visualization
                if !vial_mg().is_empty() && !bac_ml().is_empty() && !sample_mg().is_empty() && !vol().is_empty() {

                    // Result display
                    h2 {
                        class: "mt-8 md:mt-6 text-xl md:text-2xl",
                        { format!("{} ", t!("result-text-part1")) }
                        if !sample_mg().is_empty() {
                            "{sample_mg()} mg"
                            if let Ok(val) = sample_mg().parse::<f64>() {
                                if val <= 1.0 {
                                    " ({val * 1000.0} μg)"
                                }
                            }
                        } else {
                            { t!("selected-mg") }
                        }
                        { t!("result-text-part2") }
                        if let Some(result) = calculate_result() {
                            " {result}"
                        }
                        span {
                            class: "hidden md:inline-block",
                            ":"
                        }
                        if calculate_result().is_some() {
                            span {
                                class: "inline-block md:hidden",
                                { t!("result-text-mobile") }
                            }
                        } else {
                            span {
                                class: "inline-block md:hidden",
                                ":"
                            }
                        }
                    }

                    div {
                        class: "mt-8 mb-4",
                        style: "max-width: 850px;",
                        img {
                            src: get_ruler_image(),
                            alt: { t!("ruler-alt-text") }
                        }
                        if calculate_result().is_some() {
                            div {
                                class: "bg-blue-300 opacity-75 h-12",
                                style: format!(
                                    "margin-top: -{}px; width: {}%; {}",
                                    if vol() == "0.3" { "80" } else { "64" },
                                    calculate_ruler_percentage(),
                                    if calculate_ruler_percentage() >= 100.0 { "background-color: red !important;" } else { "" }
                                )
                            }
                        }
                    }

                    if calculate_ruler_percentage() >= 100.0 {
                        p {
                            class: "text-red-500",
                            { t!("not-large-enough") }
                        }
                    }
                }
            }
        }
    }
}
