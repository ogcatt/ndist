use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct SmilesDrawerProps {
    pub smiles: ReadOnlySignal<String>,
    #[props(default = 475)]
    pub height: u32,
    #[props(default = 475)]
    pub width: u32,
    #[props(default = 30)]
    pub padding: u32,
    #[props(default = "light".to_string())]
    pub theme: String,
}

pub fn SmilesViewer(props: SmilesDrawerProps) -> Element {
    let canvas_id = format!(
        "smiles-canvas-{}",
        use_hook(|| {
            static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        })
    );

    // Draw the molecule when smiles changes
    {
        let smiles = props.smiles.clone();
        let canvas_id_for_effect = canvas_id.clone();
        let theme = props.theme.clone();
        let width = props.width;
        let height = props.height;
        let padding = props.padding;

        use_effect(move || {
            if !smiles().is_empty() {
                let draw_code = format!(
                    r#"
                    const drawSmiles = () => {{
                        if (typeof SmilesDrawer !== 'undefined' && window.getSmilesDrawer) {{
                            const svgElement = document.getElementById('{}');
                            if (!svgElement) {{
                                console.error('SVG element not found: {}');
                                return;
                            }}

                            try {{
                                // Get the shared drawer instance for this size
                                const drawer = window.getSmilesDrawer({}, {}, {});

                                // Clear any existing content
                                svgElement.innerHTML = '';

                                // Parse and draw the SMILES string
                                // v2.1.7 API: drawer.draw(tree, svgElement, theme)
                                SmilesDrawer.parse('{}', function(tree) {{
                                    // IMPORTANT: Pass the element directly, not the ID
                                    drawer.draw(tree, svgElement, '{}');

                                    // Trigger fade-in effect
                                    setTimeout(() => {{
                                        if (svgElement) {{
                                            svgElement.style.opacity = '1';
                                        }}
                                    }}, 50);
                                }}, function(err) {{
                                    console.error('Error parsing SMILES for {}: ', err);
                                }});
                            }} catch (error) {{
                                console.error('Error drawing SMILES for {}: ', error);
                            }}
                        }} else {{
                            // Retry if not loaded yet
                            setTimeout(drawSmiles, 100);
                        }}
                    }};
                    drawSmiles();
                    "#,
                    canvas_id_for_effect,
                    canvas_id_for_effect,
                    width,
                    height,
                    padding,
                    smiles().replace('\'', "\\'").replace('\\', "\\\\").replace('\n', "\\n"),
                    theme,
                    canvas_id_for_effect,
                    canvas_id_for_effect,
                );
                document::eval(&draw_code);
            }
        });
    }

    rsx! {
        div {
            class: "smiles-viewer-container mol",
            style: "width: 100%; height: 100%; display: flex; align-items: center; justify-content: center; overflow: hidden;",
            svg {
                id: "{canvas_id}",
                width: "100%",
                height: "100%",
                style: "
                    max-width: 100%;
                    max-height: 100%;
                    display: block;
                    opacity: 0;
                    transition: opacity 0.3s ease-in-out;
                    user-select: none;
                    -webkit-user-select: none;
                    -moz-user-select: none;
                    -ms-user-select: none;
                    pointer-events: none;
                ",
                "data-smiles": "{props.smiles}",
                preserve_aspect_ratio: "xMidYMid meet",
                view_box: "0 0 {props.width} {props.height}"
            }
        }
    }
}
