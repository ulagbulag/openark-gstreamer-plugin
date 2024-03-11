use gsark_common::element;
use gst::{
    subclass::{prelude::ElementImpl, ElementMetadata},
    PadTemplate,
};
use once_cell::sync::Lazy;

impl ElementImpl for crate::plugin::Plugin {
    fn metadata() -> Option<&'static ElementMetadata> {
        static ELEMENT_METADATA: Lazy<ElementMetadata> = Lazy::new(|| {
            ElementMetadata::new(
                crate::metadata::LONG_NAME,
                crate::metadata::CLASS,
                crate::metadata::DESCRIPTION,
                crate::metadata::AUTHORS,
            )
        });

        Some(&*ELEMENT_METADATA)
    }

    fn pad_templates() -> &'static [PadTemplate] {
        static PAD_TEMPLATES: Templates = Templates::new(|| vec![element::sink_dynamic()]);

        PAD_TEMPLATES.as_ref()
    }
}

type Templates = Lazy<Vec<PadTemplate>>;
