pub mod base;
pub mod network;

use gst::{glib::subclass::types::ObjectSubclass, DebugCategory};

pub trait PluginImpl
where
    Self: ObjectSubclass,
{
    fn cat(&self) -> DebugCategory;
}
