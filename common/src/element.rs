use gst::{Caps, PadDirection, PadPresence, PadTemplate};

pub fn sink_dynamic() -> PadTemplate {
    PadTemplate::new(
        "sink",
        PadDirection::Sink,
        PadPresence::Always,
        &Caps::new_any(),
    )
    .unwrap()
}

pub fn src_dynamic() -> PadTemplate {
    PadTemplate::new(
        "src",
        PadDirection::Src,
        PadPresence::Always,
        &Caps::new_any(),
    )
    .unwrap()
}
