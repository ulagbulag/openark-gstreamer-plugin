use std::fmt;

use gst::{
    glib::{subclass::types::ObjectSubclassExt, value::FromValue, Value},
    info,
};

use crate::plugin::PluginImpl;

pub fn set_value<'a, P, T>(plugin: &P, name: &str, field: &mut T, value: &'a Value)
where
    P: ObjectSubclassExt + PluginImpl,
    T: fmt::Debug + FromValue<'a>,
{
    let value = value.get().expect("type checked upstream");
    info!(
        plugin.cat(),
        imp: plugin,
        "Changing {name} from {field:?} to {value:?}",
    );
    *field = value;
}
