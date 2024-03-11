mod args;
mod element;
mod plugin;

use gst::{glib, prelude::StaticType, DebugCategory, DebugColorFlags, Element, Object, Rank};
use once_cell::sync::Lazy;

pub mod metadata {
    pub const CLASS: &str = "Source/Network";
    pub const CLASS_NAME: &str = "ArkSrc";
    pub const LONG_NAME: &str = "OpenARK message receiver";

    pub const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
    pub const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");
    pub const NAME: &str = env!("CARGO_PKG_NAME");
}

// The public Rust wrapper type for our element
glib::wrapper! {
    pub struct Plugin(ObjectSubclass<plugin::Plugin>)
    @extends
        gst_base::PushSrc,
        gst_base::BaseSrc,
        Element,
        Object
    ;
}

/// Registers the type for our element, and then registers in GStreamer under
/// the name for being able to instantiate it via e.g.
/// gst::ElementFactory::make().
pub fn register(plugin: &gst::Plugin) -> Result<(), glib::BoolError> {
    Element::register(
        Some(plugin),
        crate::metadata::NAME,
        Rank::NONE,
        Plugin::static_type(),
    )
}

// This module contains the private implementation details of our element
//
pub(crate) static CAT: Lazy<DebugCategory> = Lazy::new(|| {
    DebugCategory::new(
        crate::metadata::NAME,
        DebugColorFlags::empty(),
        Some(crate::metadata::DESCRIPTION),
    )
});
