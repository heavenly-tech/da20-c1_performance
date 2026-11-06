//! AFM-style labels at guide line ends (legible on all themes/palettes).

use leptos::prelude::*;

#[derive(Clone, Copy)]
pub enum LabelAnchor {
    Start,
    Middle,
    End,
}

pub fn guide_line_label(x: f64, y: f64, text: String, anchor: LabelAnchor) -> impl IntoView {
    let anchor_str = match anchor {
        LabelAnchor::Start => "start",
        LabelAnchor::Middle => "middle",
        LabelAnchor::End => "end",
    };
    view! {
        <text
            x=x
            y=y
            class="chart-guide-label"
            font-size="9"
            text-anchor=anchor_str
            dominant-baseline="middle"
        >
            {text}
        </text>
    }
}
