use super::front_entities::{ShippingOption, ShippingQuote, ShippingResult};
use crate::utils::countries::allowed_countries;
use dioxus::prelude::*;

// ADDS dynamic fee based on basket cost to all US orders
const US_DYNAMIC_FEE: bool = true;

const DISABLE_US_TRACKED: bool = false;

// Weight brackets in grams for tracked shipping
const TRACKED_WEIGHTS: &[u32] = &[
    50, 75, 100, 125, 150, 175, 200, 225, 250, 275, 300, 325, 350, 375, 400, 425, 450, 475, 500,
    550, 600, 650, 700, 750, 800, 850, 900, 950, 1000, 1100, 1200, 1300, 1400, 1500, 1600, 1700,
    1800, 1900, 2000,
];

// Tracked shipping prices by zone (EUR)
const TRACKED_PRICES: &[&[f64]] = &[
    // Zone 1 (Eastern Europe)
    &[
        4.28, 4.37, 4.52, 4.75, 4.94, 5.13, 5.18, 5.23, 5.28, 5.32, 5.42, 5.51, 5.61, 5.70, 5.75,
        5.80, 5.85, 5.89, 5.94, 6.18, 6.18, 6.18, 6.18, 6.18, 6.18, 6.18, 6.18, 6.18, 6.18, 6.65,
        6.65, 6.65, 6.65, 6.65, 6.65, 6.65, 6.65, 6.65, 6.65,
    ],
    // Zone 2 (Germany)
    &[
        4.85, 4.90, 4.99, 5.09, 5.18, 5.42, 5.61, 5.70, 5.80, 5.89, 5.99, 6.08, 6.18, 6.27, 6.37,
        6.46, 6.46, 6.65, 6.75, 6.89, 7.08, 7.27, 7.46, 7.60, 7.84, 8.08, 8.32, 8.55, 8.79, 9.03,
        9.27, 9.50, 9.74, 9.98, 10.22, 10.45, 10.69, 10.93, 11.17,
    ],
    // Zone 3 (France)
    &[
        5.04, 5.09, 5.13, 5.28, 5.64, 6.01, 6.12, 6.22, 6.38, 6.54, 6.80, 7.01, 7.22, 7.42, 7.53,
        7.74, 7.95, 8.16, 8.36, 8.78, 8.99, 9.20, 9.41, 9.93, 10.45, 10.72, 10.98, 11.24, 10.98,
        11.88, 12.59, 13.07, 13.54, 14.02, 14.49, 14.97, 15.44, 15.92, 16.39,
    ],
    // Zone 4 (Italy)
    &[
        5.37, 5.47, 5.56, 5.61, 5.70, 5.94, 6.18, 6.42, 6.18, 5.94, 6.18, 6.42, 6.65, 6.84, 7.03,
        7.22, 7.41, 7.60, 7.84, 8.08, 8.32, 8.55, 8.79, 9.03, 9.27, 9.50, 9.74, 9.98, 10.22, 9.98,
        10.45, 10.93, 11.40, 11.88, 12.35, 12.83, 13.30, 13.78, 14.25,
    ],
    // Zone 5 (Western Europe)
    &[
        5.09, 5.19, 5.39, 5.54, 5.64, 5.70, 5.75, 5.79, 5.85, 5.94, 6.18, 6.37, 6.56, 6.75, 6.84,
        7.03, 7.22, 7.41, 7.60, 7.98, 8.17, 8.36, 8.55, 9.03, 9.50, 9.74, 9.98, 10.22, 9.98, 10.45,
        10.93, 11.40, 11.88, 12.35, 12.83, 13.30, 13.78, 14.25, 14.73,
    ],
    // Zone 6 (Other EU)
    &[
        5.65, 5.77, 5.90, 6.02, 6.15, 6.27, 6.40, 6.53, 6.65, 6.78, 6.90, 7.03, 7.15, 7.28, 7.40,
        7.53, 7.65, 7.78, 7.91, 8.32, 8.74, 9.16, 9.58, 10.00, 10.41, 10.83, 11.25, 11.67, 12.09,
        12.61, 13.13, 13.65, 14.18, 14.70, 15.22, 15.74, 16.27, 16.79, 17.31,
    ],
    // Zone 7 (Great Britain)
    &[
        4.94, 5.09, 5.23, 5.37, 5.51, 5.66, 5.80, 5.94, 6.08, 6.23, 6.37, 6.51, 6.99, 7.14, 7.29,
        7.44, 7.59, 7.74, 7.89, 8.23, 8.58, 8.93, 9.28, 9.63, 9.98, 10.33, 10.68, 11.03, 11.38,
        11.88, 12.37, 12.87, 13.37, 13.87, 14.37, 14.87, 15.37, 15.87, 16.36,
    ],
    // Zone 8 - Switzerland & Norway
    &[
        5.89, 5.99, 6.19, 6.35, 6.61, 6.77, 7.02, 7.22, 7.43, 7.60, 7.85, 8.01, 8.67, 8.85, 9.11,
        9.29, 9.55, 9.72, 9.99, 10.42, 10.86, 11.30, 11.73, 12.16, 12.60, 13.04, 13.47, 13.91,
        14.35, 15.22, 16.09, 16.96, 17.83, 18.71, 19.58, 20.45, 21.32, 22.20, 23.07,
    ],
    // Zone 9 (North America)
    &[
        5.85, 6.11, 6.49, 6.74, 7.84, 8.12, 8.54, 8.89, 9.23, 9.52, 9.94, 10.22, 10.64, 10.91,
        11.34, 11.61, 12.04, 13.99, 14.21, 13.78, 14.25, 14.73, 15.20, 15.92, 16.63, 17.34, 18.05,
        18.77, 19.74, 20.66, 22.03, 23.40, 24.77, 26.13, 27.50, 28.87, 30.25, 31.61, 32.98,
    ],
    // Zone 10 (Australia, New Zealand)
    &[
        7.11, 7.49, 7.87, 8.25, 8.64, 9.02, 9.40, 9.78, 10.17, 10.55, 10.93, 11.31, 11.69, 12.08,
        12.46, 12.84, 13.22, 13.61, 13.99, 14.81, 15.63, 16.45, 17.27, 18.09, 18.91, 19.72, 20.54,
        21.36, 22.18, 23.82, 25.46, 27.10, 28.74, 30.38, 32.02, 33.65, 35.29, 36.93, 38.57,
    ],
    // Zone 11 (Rest of World)
    &[
        6.43, 6.72, 7.01, 7.41, 7.81, 8.26, 8.72, 9.18, 9.64, 10.10, 10.56, 11.02, 11.48, 11.94,
        12.39, 12.85, 13.31, 13.77, 14.23, 15.23, 16.22, 17.22, 18.22, 19.22, 20.21, 21.21, 22.21,
        23.21, 24.20, 25.20, 26.20, 27.20, 28.19, 29.19, 30.19, 31.19, 32.18, 33.18, 34.18,
    ],
];

// TrackedUS weights and prices
const TRACKEDUS_WEIGHTS: &[u32] = &[
    50, 75, 100, 125, 150, 175, 200, 225, 250, 275, 300, 325, 350, 375, 400, 425, 450, 475, 500,
    550, 600, 650, 700, 750, 800, 850, 900, 950, 1000, 1100, 1200, 1300, 1400, 1500, 1600, 1700,
    1800, 1900, 2000, 2500, 3000, 3500, 4000, 4500, 5000, 5500, 6000,
];

const TRACKEDUS_PRICES: &[f64] = &[
    6.80, 7.21, 7.66, 7.95, 8.42, 8.71, 9.16, 9.54, 9.91, 10.22, 10.66, 10.97, 11.42, 11.72, 12.17,
    12.46, 13.25, 14.00, 14.45, 14.70, 15.55, 16.20, 16.68, 17.44, 18.18, 18.94, 19.69, 20.45,
    21.20, 22.69, 24.20, 25.71, 27.21, 28.71, 30.22, 31.72, 33.23, 34.73, 36.23, 41.46, 46.69,
    51.92, 57.14, 62.37, 67.60, 72.83, 78.05,
];

// Express shipping pricing structure
struct ExpressPricing {
    packet_price: f64, // EUR
    parcel_price: f64, // EUR
}

fn get_express_pricing(country_code: &str) -> Option<ExpressPricing> {
    match country_code {
        "US"/*| "CA"*/ => Some(ExpressPricing {
            packet_price: 31.53,
            parcel_price: 35.95,
        }),
        "GB" => Some(ExpressPricing {
            packet_price: 40.49,
            parcel_price: 43.63,
        }),
        "AU" => Some(ExpressPricing {
            packet_price: 65.54,
            parcel_price: 74.84,
        }),
        "DE" => Some(ExpressPricing {
            packet_price: 26.74,
            parcel_price: 29.81,
        }),
        "CH" => Some(ExpressPricing {
            packet_price: 29.4,
            parcel_price: 33.59,
        }),
        "IT" => Some(ExpressPricing {
            packet_price: 78.97,
            parcel_price: 90.16,
        }),
        "SE" => Some(ExpressPricing {
            packet_price: 78.97,
            parcel_price: 90.16,
        }),
        "IE" => Some(ExpressPricing {
            packet_price: 78.97,
            parcel_price: 90.16,
        }),
        "GR" => Some(ExpressPricing {
            packet_price: 26.74,
            parcel_price: 29.81,
        }),
        "FR" => Some(ExpressPricing {
            packet_price: 26.74,
            parcel_price: 29.81,
        }),
        "ES" => Some(ExpressPricing {
            packet_price: 27.29,
            parcel_price: 30.43,
        }),
        "PT" => Some(ExpressPricing {
            packet_price: 29.4,
            parcel_price: 33.59,
        }),
        "AT" => Some(ExpressPricing {
            packet_price: 27.29,
            parcel_price: 30.43,
        }),
        "HU" => Some(ExpressPricing {
            packet_price: 26.74,
            parcel_price: 29.81,
        }),
        "IS" => Some(ExpressPricing {
            packet_price: 56.1,
            parcel_price: 63.49,
        }),
        "BE" => Some(ExpressPricing {
            packet_price: 26.74,
            parcel_price: 29.81,
        }),
        "NL" => Some(ExpressPricing {
            packet_price: 26.74,
            parcel_price: 29.81,
        }),
        "LU" => Some(ExpressPricing {
            packet_price: 26.74,
            parcel_price: 29.81,
        }),
        "DK" => Some(ExpressPricing {
            packet_price: 27.29,
            parcel_price: 30.43,
        }),
        "NO" => Some(ExpressPricing {
            packet_price: 78.97,
            parcel_price: 90.16,
        }),
        "PL" => Some(ExpressPricing {
            packet_price: 27.29,
            parcel_price: 30.43,
        }),
        "CZ" => Some(ExpressPricing {
            packet_price: 78.97,
            parcel_price: 90.16,
        }),
        "SI" => Some(ExpressPricing {
            packet_price: 26.74,
            parcel_price: 29.81,
        }),
        "HR" => Some(ExpressPricing {
            packet_price: 27.29,
            parcel_price: 30.43,
        }),
        "ME" => Some(ExpressPricing {
            packet_price: 29.4,
            parcel_price: 33.59,
        }),
        "RS" => Some(ExpressPricing {
            packet_price: 78.97,
            parcel_price: 90.16,
        }),
        "MK" => Some(ExpressPricing {
            packet_price: 29.4,
            parcel_price: 33.59,
        }),
        "RO" => Some(ExpressPricing {
            packet_price: 26.74,
            parcel_price: 29.81,
        }),
        "LT" => Some(ExpressPricing {
            packet_price: 78.97,
            parcel_price: 90.16,
        }),
        "LV" => Some(ExpressPricing {
            packet_price: 78.97,
            parcel_price: 90.16,
        }),
        "EE" => Some(ExpressPricing {
            packet_price: 78.97,
            parcel_price: 90.16,
        }),
        "FI" => Some(ExpressPricing {
            packet_price: 29.4,
            parcel_price: 33.59,
        }),
        "MD" => Some(ExpressPricing {
            packet_price: 29.4,
            parcel_price: 33.59,
        }),
        "CY" => Some(ExpressPricing {
            packet_price: 56.1,
            parcel_price: 63.49,
        }),
        "NZ" => Some(ExpressPricing {
            packet_price: 65.54,
            parcel_price: 74.84,
        }),
        "JP" => Some(ExpressPricing {
            packet_price: 56.64,
            parcel_price: 64.72,
        }),
        "KR" => Some(ExpressPricing {
            packet_price: 56.64,
            parcel_price: 64.72,
        }),
        "CN" => Some(ExpressPricing {
            packet_price: 56.64,
            parcel_price: 64.72,
        }),
        _ => None,
    }
}

fn get_country_zone(country_code: &str) -> Option<usize> {
    match country_code {
        // Zone 1 - Eastern Europe
        "RO" | "SK" | "CZ" | "HU" | "GR" | "PL" | "HR" | "SI" | "CY" => Some(0),
        // Zone 2 - Germany
        "DE" => Some(1),
        // Zone 3 - France
        "FR" => Some(2),
        // Zone 4 - Italy
        "IT" => Some(3),
        // Zone 5 - Western Europe
        "AT" | "BE" | "NL" | "ES" => Some(4),
        // Zone 6 - Other EU countries
        "DK" | "FI" | "SE" | "IE" | "LU" | "PT" | "EE" | "LV" | "LT" => Some(5),
        // Zone 7 - Great Britain
        "GB" => Some(6),
        // Zone 8 - Switzerland & Norway
        "CH" | "NO" => Some(7),
        // Zone 9 - North America
        "US" | "CA" => Some(8),
        // Zone 10 - Australia, New Zealand
        "AU" | "NZ" => Some(9),
        // Zone 11 - Rest of World (all other countries)
        _ => Some(10),
    }
}

fn is_eu_country(country_code: &str) -> bool {
    matches!(
        country_code,
        "AT" | "BE"
            | "BG"
            | "HR"
            | "CY"
            | "CZ"
            | "DK"
            | "EE"
            | "FI"
            | "FR"
            | "DE"
            | "GR"
            | "HU"
            | "IE"
            | "IT"
            | "LV"
            | "LT"
            | "LU"
            | "MT"
            | "NL"
            | "PL"
            | "PT"
            | "RO"
            | "SK"
            | "SI"
            | "ES"
            | "SE"
    )
}

fn find_price_bracket(weight_grams: u32, weights: &[u32]) -> Option<usize> {
    for (i, &bracket_weight) in weights.iter().enumerate() {
        if weight_grams <= bracket_weight {
            return Some(i);
        }
    }
    None
}

fn get_shipping_time(country_code: &str, option: &ShippingOption) -> String {
    match option {
        ShippingOption::Express => "2-4".to_string(),
        ShippingOption::TrackedUS => "6-8".to_string(),
        ShippingOption::Tracked => {
            match country_code {
                // Zone 1 - Eastern Europe
                "RO" => "3-7",
                "SK" | "CZ" | "HU" | "GR" => "4-6",
                "PL" | "HR" | "SI" => "5-7",
                "CY" => "6-8",
                // Zone 2 - Germany
                "DE" => "3-5",
                // Zone 3 - France
                "FR" => "4-6",
                // Zone 4 - Italy
                "IT" => "4-6",
                // Zone 5 - Western Europe
                "AT" | "BE" | "NL" => "4-6",
                "ES" => "5-7",
                // Zone 6 - Other EU
                "DK" | "LU" => "4-6",
                "FI" | "SE" | "IE" | "PT" | "EE" | "LV" | "LT" => "5-7",
                // Zone 7 - Great Britain
                "GB" => "5-7",
                // Zone 8 - Switzerland & Norway
                "CH" => "5-7",
                "NO" => "6-8",
                // Zone 9 - North America
                "US" => "8-14",
                "CA" => "8-12",
                // Zone 10 - Australia, New Zealand
                "AU" => "8-12",
                "NZ" => "8-14",
                //Domestic
                "BG" => "1-2",
                // Zone 11 - Rest of World
                _ => "~10-14",
            }
            .to_string()
        }
    }
}

fn round_to_2_decimals(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

pub fn round_up_to_nearest_quarter(value: f64) -> f64 {
    (value * 4.0).ceil() / 4.0
}

pub fn calculate_shipping_cost(
    country_code: &str,
    weight_grams: u32,
    basket_cost_usd: f64,
) -> Option<ShippingQuote> {
    let country_code = country_code.to_uppercase();

    // Check if country is in allowed countries
    if !allowed_countries().contains(&country_code.as_str()) {
        return None;
    }

    let mut available_options = Vec::new();

    // Calculate Tracked shipping
    if country_code == "US" {
        // Use TrackedUS for US

        // Check if US tracked is disabled or not
        if !DISABLE_US_TRACKED {
            if let Some(bracket_index) = find_price_bracket(weight_grams, TRACKEDUS_WEIGHTS) {
                let mut price_eur = TRACKEDUS_PRICES[bracket_index];

                // Add 20% for EU countries and GB (US is not EU, so no tax)
                let mut price_usd = price_eur * 1.2; // EUR to USD conversion

                if US_DYNAMIC_FEE {
                    // Add dynamic fee: basket_cost * 0.16 + 1.5 EUR (converted to USD)
                    let fee_usd = basket_cost_usd * 0.16 + (1.5 * 1.2); // 1.5 EUR to USD
                    price_usd += fee_usd;
                }

                available_options.push(ShippingResult {
                    option: ShippingOption::TrackedUS,
                    cost_usd: round_up_to_nearest_quarter(price_usd),
                    estimated_days: get_shipping_time(&country_code, &ShippingOption::TrackedUS),
                });
            }
        }
    } else {
        // Use regular Tracked shipping
        if let (Some(zone), Some(bracket_index)) = (
            get_country_zone(&country_code),
            find_price_bracket(weight_grams, TRACKED_WEIGHTS),
        ) {
            let mut price_eur = if country_code == "BG" {
                2.42
            } else {
                TRACKED_PRICES[zone][bracket_index]
            };

            // Add 20% for EU countries and GB
            if is_eu_country(&country_code) || country_code == "GB" {
                price_eur *= 1.2;
            }

            let price_usd = price_eur * 1.2; // EUR to USD conversion

            available_options.push(ShippingResult {
                option: ShippingOption::Tracked,
                cost_usd: round_up_to_nearest_quarter(price_usd),
                estimated_days: get_shipping_time(&country_code, &ShippingOption::Tracked),
            });
        }
    }

    // Calculate Express shipping with new logic
    if weight_grams <= 980 {
        // Maximum weight for express (parcel limit)
        if let Some(express_pricing) = get_express_pricing(&country_code) {
            let mut price_eur = if weight_grams < 300 {
                express_pricing.packet_price // Packet: <300g
            } else {
                express_pricing.parcel_price // Parcel: 300-980g
            };

            // Add 20% VAT for EU countries and GB
            if is_eu_country(&country_code) || country_code == "GB" {
                price_eur *= 1.2;
            }

            // Convert EUR to USD (approx)
            let mut price_usd = price_eur * 1.2;

            if US_DYNAMIC_FEE && country_code == "US" {
                // Add dynamic fee: basket_cost * 0.16 + 1.5 EUR (converted to USD)
                let fee_usd = basket_cost_usd * 0.16 + (1.5 * 1.2); // 1.5 EUR to USD
                price_usd += fee_usd;
            }

            available_options.push(ShippingResult {
                option: ShippingOption::Express,
                cost_usd: round_up_to_nearest_quarter(price_usd),
                estimated_days: get_shipping_time(&country_code, &ShippingOption::Express),
            });
        }
    }

    if available_options.is_empty() {
        None
    } else {
        Some(ShippingQuote { available_options })
    }
}

// New function to calculate shipping cost with pre-order surcharge
pub fn calculate_shipping_cost_with_preorder_surcharge(
    country_code: &str,
    weight_grams: u32,
    basket_cost_usd: f64,
) -> Option<ShippingQuote> {
    if let Some(mut shipping_quote) =
        calculate_shipping_cost(country_code, weight_grams, basket_cost_usd)
    {
        // Add 10% surcharge to Express shipping for pre-orders
        for shipping_result in &mut shipping_quote.available_options {
            if matches!(shipping_result.option, ShippingOption::Express) {
                shipping_result.cost_usd =
                    round_up_to_nearest_quarter(shipping_result.cost_usd * 1.05);
                /* The above 5% added is to compensate for the potential higher delivery prices as this is delayed dispatch */
            }
        }
        Some(shipping_quote)
    } else {
        None
    }
}

/*
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_express_packet_vs_parcel() {
        // Test packet pricing (<300g)
        let quote = calculate_shipping_cost("DE", 250, 100.0);
        if let Some(quote) = quote {
            let express_option = quote.available_options.iter()
                .find(|opt| matches!(opt.option, ShippingOption::Express));
            assert!(express_option.is_some());
        }

        // Test parcel pricing (300-980g)
        let quote = calculate_shipping_cost("DE", 500, 100.0);
        if let Some(quote) = quote {
            let express_option = quote.available_options.iter()
                .find(|opt| matches!(opt.option, ShippingOption::Express));
            assert!(express_option.is_some());
        }

        // Test over weight limit (>980g) - should not have express
        let quote = calculate_shipping_cost("DE", 1200, 100.0);
        if let Some(quote) = quote {
            let express_option = quote.available_options.iter()
                .find(|opt| matches!(opt.option, ShippingOption::Express));
            assert!(express_option.is_none());
        }
    }

    #[test]
    fn test_us_uses_trackedus() {
        let quote = calculate_shipping_cost("US", 500, 100.0);
        if let Some(quote) = quote {
            let tracked_us_option = quote.available_options.iter()
                .find(|opt| matches!(opt.option, ShippingOption::TrackedUS));
            assert!(tracked_us_option.is_some());
        }
    }

    #[test]
    fn test_us_dynamic_fee() {
        let basket_cost = 100.0; // $100 basket
        let quote = calculate_shipping_cost("US", 500, basket_cost);
        if let Some(quote) = quote {
            // Should have dynamic fee applied (16% of basket + 1.5 EUR converted to USD)
            // Expected fee: 100 * 0.16 + 1.5 * 1.2 = 16 + 1.8 = 17.8 USD
            assert!(!quote.available_options.is_empty());
        }
    }
}
*/
