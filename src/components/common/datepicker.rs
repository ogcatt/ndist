use chrono::NaiveDateTime;
use dioxus::prelude::*;

#[component]
pub fn CDatePicker(
    label: String,
    value: Option<NaiveDateTime>,
    optional: bool,
    oninput: EventHandler<Option<NaiveDateTime>>,
) -> Element {
    let date_string = if let Some(date) = value {
        date.format("%Y-%m-%d").to_string()
    } else {
        String::new()
    };

    rsx! {
        div {
            class: "flex flex-col gap-1",
            label {
                class: format!("text-sm font-medium text-gray-700 {}",
                    if !optional { "after:content-['*'] after:text-red-500 after:ml-1" } else { "" }
                ),
                "{label}"
            }
            input {
                r#type: "date",
                class: "px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500 focus:border-blue-500",
                value: "{date_string}",
                oninput: move |evt: FormEvent| {
                    let value = evt.value();
                    if value.is_empty() {
                        oninput.call(None);
                    } else if let Ok(parsed_date) = chrono::NaiveDate::parse_from_str(&value, "%Y-%m-%d") {
                        // Set time to end of day (23:59:59)
                        let datetime = parsed_date.and_hms_opt(23, 59, 59).unwrap_or_else(|| {
                            parsed_date.and_hms_opt(0, 0, 0).unwrap()
                        });
                        oninput.call(Some(datetime));
                    }
                }
            }
        }
    }
}
