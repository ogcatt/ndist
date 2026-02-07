use dioxus::prelude::*;
use dioxus_i18n::t;

#[component]
pub fn ShippingMap(country: ReadOnlySignal<String>, canvas_width: i32) -> Element {
    let domestic_countries = vec!["BG"];
    let europe_list = vec![
        "GB", "IE", "IT", "HR", "ES", "FR", "DE", "BE", "NL", "LU", "NL", "AT", "SL", "SK", "HU",
        "CZ", "SE", "NO", "FI", "DK", "MC", "AD", "PT", "BA", "IS", "RS", "XK", "ME", "AL", "MK",
        "GR", "PL", "CH", "UA", "MD", "BY",
    ];

    tracing::info!("Initial shipping map rendering: {:?}", country());

    let eu_list = europe_list.clone();

    let locations = vec![
        Location {
            countries: vec!["GB", "IE"],
            x: [295.0, 155.0],
            y: [150.0, 98.0],
        },
        Location {
            countries: vec!["IT", "HR", "SL"],
            x: [295.0, 214.0],
            y: [150.0, 137.0],
        },
        Location {
            countries: vec!["ES"],
            x: [295.0, 110.0],
            y: [150.0, 167.0],
        },
        Location {
            countries: vec!["FR", "DE", "BE", "NL", "LU", "MC", "AD"],
            x: [295.0, 188.0],
            y: [150.0, 118.0],
        },
        Location {
            countries: vec!["AT", "SK", "HU", "CZ"],
            x: [295.0, 250.0],
            y: [150.0, 125.0],
        },
        Location {
            countries: vec!["SE"],
            x: [295.0, 240.0],
            y: [150.0, 60.0],
        },
        Location {
            countries: vec!["NO"],
            x: [295.0, 214.0],
            y: [150.0, 55.0],
        },
        Location {
            countries: vec!["FI"],
            x: [295.0, 280.0],
            y: [150.0, 45.0],
        },
        Location {
            countries: vec!["DK"],
            x: [295.0, 212.0],
            y: [150.0, 81.0],
        },
        Location {
            countries: vec!["PT"],
            x: [295.0, 76.0],
            y: [150.0, 176.0],
        },
        Location {
            countries: vec!["BA", "RS", "XK", "ME", "AL"],
            x: [295.0, 270.0],
            y: [150.0, 150.0],
        },
        Location {
            countries: vec!["IS"],
            x: [295.0, 134.0],
            y: [150.0, 23.0],
        },
        Location {
            countries: vec!["MK"],
            x: [295.0, 285.0],
            y: [150.0, 160.0],
        },
        Location {
            countries: vec!["GR"],
            x: [295.0, 283.0],
            y: [150.0, 172.0],
        },
        Location {
            countries: vec!["PL"],
            x: [295.0, 255.0],
            y: [150.0, 110.0],
        },
        Location {
            countries: vec!["CH"],
            x: [295.0, 193.0],
            y: [150.0, 134.0],
        },
        Location {
            countries: vec!["US"],
            x: [210.0, 65.0],
            y: [52.0, 58.0],
        },
        Location {
            countries: vec!["CA"],
            x: [210.0, 70.0],
            y: [52.0, 40.0],
        },
        Location {
            countries: vec!["MX"],
            x: [210.0, 50.0],
            y: [52.0, 80.0],
        },
        Location {
            countries: vec![
                "AR", "AW", "BZ", "BO", "BR", "CL", "CO", "CR", "CU", "DO", "EC", "SV", "FK", "GF",
                "GT", "GY", "HT", "HN", "MX", "AN", "NI", "PA", "PY", "PE", "GS", "SR", "TT", "UY",
                "VE",
            ],
            x: [210.0, 108.0],
            y: [52.0, 136.0],
        },
        Location {
            countries: vec![
                "DZ", "AO", "BJ", "BW", "BF", "BI", "CM", "CV", "CF", "TD", "KM", "CG", "CD", "CI",
                "DJ", "EG", "GQ", "ER", "ET", "GA", "GM", "GH", "GN", "GW", "KE", "LS", "LR", "LY",
                "MG", "ML", "MW", "MR", "MU", "YT", "MA", "MZ", "NA", "NE", "NG", "RE", "RW", "ST",
                "SN", "SC", "SL", "SO", "ZA", "SS", "SD", "SZ", "TZ", "TG", "TN", "UG", "EH", "ZM",
                "ZW",
            ],
            x: [210.0, 205.0],
            y: [52.0, 95.0],
        },
        Location {
            countries: vec!["AU"],
            x: [210.0, 350.0],
            y: [52.0, 155.0],
        },
        Location {
            countries: vec!["NZ"],
            x: [210.0, 390.0],
            y: [52.0, 180.0],
        },
        Location {
            countries: vec!["RU"],
            x: [210.0, 260.0],
            y: [52.0, 38.0],
        },
        Location {
            countries: vec!["UA", "MD", "BY"],
            x: [295.0, 330.0],
            y: [150.0, 114.0],
        },
        Location {
            countries: vec!["JP"],
            x: [210.0, 347.0],
            y: [52.0, 65.0],
        },
        Location {
            countries: vec!["GL"],
            x: [210.0, 150.0],
            y: [52.0, 10.0],
        },
    ];

    // These are now computed values that automatically update when country changes
    let country_state = use_memo(move || country().to_uppercase());
    let canvas_height = use_memo(move || {
        let in_europe = eu_list.contains(&country_state().as_str());
        if in_europe {
            canvas_width as f32 / 1.45
        } else {
            canvas_width as f32 / 1.9
        }
    });

    let country_data = use_memo(move || {
        let country_upper = country_state();
        for loc in &locations {
            if loc.countries.contains(&country_upper.as_str()) {
                return loc.clone();
            }
        }
        // Default fallback
        Location {
            x: [210.0, 290.0],
            y: [52.0, 60.0],
            countries: vec![],
        }
    });

    let begin = use_memo(move || {
        let data = country_data();
        Point {
            x: (data.x[0] / 400.0) * canvas_width as f32,
            y: (data.y[0] / 200.0) * canvas_height(),
        }
    });

    let end = use_memo(move || {
        let data = country_data();
        Point {
            x: (data.x[1] / 400.0) * canvas_width as f32,
            y: (data.y[1] / 200.0) * canvas_height(),
        }
    });

    let path_length = use_memo(move || {
        let data = country_data();
        let dx = data.x[1] - data.x[0];
        let dy = data.y[1] - data.y[0];
        let chord = (dx * dx + dy * dy).sqrt();
        let radius = 220.0 + (canvas_width as f32 * 0.05);
        let arc_length = 2.0 * radius * (chord / (2.0 * radius)).asin();
        arc_length * (canvas_width as f32 / 400.0)
    });

    let in_europe_check = use_memo(move || europe_list.contains(&country_state().as_str()));
    let background = use_memo(move || {
        if in_europe_check() {
            format!("url('{}')", asset!("/assets/images/europe.svg"))
        } else {
            format!("url('{}')", asset!("/assets/images/world-map.png"))
        }
    });

    let background_style = use_memo(move || {
        format!(
            "background: {}; background-size: {}px {}px; width: {}px; background-repeat: no-repeat;",
            background(),
            canvas_width,
            canvas_height(),
            canvas_width
        )
    });

    let is_domestic = use_memo(move || domestic_countries.contains(&country_state().as_str()));

    if is_domestic() {
        rsx! {
            div {
                class: "text-center pb-2",
                div {
                    class: "flex justify-center py-4",
                    img {
                        class: "w-[60%] max-w-32",
                        src: asset!("/assets/icons/box.png")
                    }
                }
                p {
                    class: "text-ui-fg-subtle",
                    { t!("domestic-country-shipping", country: country_state()) }
                }
                small {
                    class: "text-center text-xs text-ui-fg-subtle",
                    { t!("in-country-delivery") }
                }
            }
        }
    } else {
        rsx! {
            div {
                style: "{background_style()}",
                svg {
                    xmlns: "http://www.w3.org/2000/svg",
                    width: "{canvas_width}",
                    height: "{canvas_height()}",
                    defs {
                        path {
                            id: "basePath",
                            d: "M {begin().x},{begin().y} A {220.0 + (canvas_width as f32 * 0.05)} 500 0 0 1 {end().x},{end().y}"
                        }
                        mask {
                            id: "mask",
                            r#use {
                                href: "#basePath",
                                stroke_width: "3",
                                stroke: "white",
                                stroke_dasharray: "1000,0",
                                fill: "none",
                                animate {
                                    attribute_name: "stroke-dasharray",
                                    from: "0,{path_length()}",
                                    to: "{path_length()},0",
                                    begin: "0s",
                                    dur: "6s",
                                    repeat_count: "indefinite"
                                }
                            }
                        }
                    }
                    r#use {
                        href: "#basePath",
                        stroke_width: "2",
                        stroke_dasharray: "10",
                        stroke: "#0091CB",
                        fill: "none",
                        mask: "url(#mask)"
                    }
                    circle {
                        cx: "{begin().x}",
                        cy: "{begin().y}",
                        r: "4",
                        fill: "gray"
                    }
                    circle {
                        cx: "{end().x}",
                        cy: "{end().y}",
                        r: "4",
                        fill: "#90ee90"
                    }
                    path {
                        d: "M 27,3 H 21 L 13,15 H 9 L 12,3 H 5 L 3,7 H -1 L 1,0 -1,-7 H 3 L 5,-3 H 12 L 9,-15 H 13 L 21,-3 H 27 C 33,-3 33,3 27,3 Z",
                        fill: "white",
                        stroke: "black",
                        stroke_width: "1.5",
                        animateMotion {
                            rotate: "auto",
                            begin: "0s",
                            dur: "6s",
                            repeat_count: "indefinite",
                            mpath {
                                href: "#basePath"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Clone, PartialEq)]
struct Location {
    pub countries: Vec<&'static str>,
    pub x: [f32; 2],
    pub y: [f32; 2],
}

#[derive(Default, Clone, PartialEq)]
struct Point {
    pub x: f32,
    pub y: f32,
}
