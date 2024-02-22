use gst::{subclass::prelude::ElementImpl, Caps};
use once_cell::sync::Lazy;

/// Implementation of gst::Element virtual methods
impl ElementImpl for crate::plugin::Plugin {
    /// Set the element specific metadata. This information is what
    /// is visible from gst-inspect-1.0 and can also be programmatically
    /// retrieved from the gst::Registry after initial registration
    /// without having to load the plugin in memory.
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

    /// Create and add pad templates for our sink and source pad. These
    /// are later used for actually creating the pads and beforehand
    /// already provide information to GStreamer about all possible
    /// pads that could exist for this type.
    ///
    /// Our element here can convert BGRx to BGRx or GRAY8, both being grayscale.
    fn pad_templates() -> &'static [gst::PadTemplate] {
        static PAD_TEMPLATES: Templates = Templates::new(|| {
            vec![{
                gst::PadTemplate::new(
                    "src",
                    gst::PadDirection::Src,
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
