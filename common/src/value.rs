use std::fmt;

use gst::{
    glib::{subclass::types::ObjectSubclassExt, value::FromValue, Value},
    DebugCategory,
};

pub(super) fn set_value<'a, Plugin, T>(
    cat: &DebugCategory,
    plugin: &Plugin,
    name: &str,
    field: &mut T,
    value: &'a Value,
) where
    Plugin: ObjectSubclassExt,
    T: fmt::Display + FromValue<'a>,
{
    let value = value.get().expect("type checked upstream");
    gst::info!(
        cat,
        imp: plugin,
        "Changing {name} from {field} to {value}",
    );
    *field = value;
}
