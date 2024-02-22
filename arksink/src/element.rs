use gst::{subclass::prelude::ElementImpl, Caps};
use once_cell::sync::Lazy;

impl ElementImpl for crate::plugin::Plugin {
    fn metadata() -> Option<&'static gst::subclass::ElementMetadata> {
        static ELEMENT_METADATA: Lazy<gst::subclass::ElementMetadata> = Lazy::new(|| {
            gst::subclass::ElementMetadata::new(
                crate::metadata::LONG_NAME,
                crate::metadata::CLASS,
                crate::metadata::DESCRIPTION,
                crate::metadata::AUTHORS,
            )
        });

        Some(&*ELEMENT_METADATA)
    }

    fn pad_templates() -> &'static [gst::PadTemplate] {
        static PAD_TEMPLATES: Templates = Templates::new(|| {
            vec![{
                gst::PadTemplate::new(
                    "sink",
                    gst::PadDirection::Sink,
                    gst::PadPresence::Always,
                    &Caps::new_any(),
                )
                .unwrap()
            }]
        });

        PAD_TEMPLATES.as_ref()
    }
}

type Templates = Lazy<Vec<gst::PadTemplate>>;
