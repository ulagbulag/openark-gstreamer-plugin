use gst::glib::{
    value::ToValue, ParamSpec, ParamSpecBoolean, ParamSpecBuilderExt, ParamSpecString, Value,
};
use once_cell::sync::Lazy;

use crate::{channel::ChannelArgs, plugin::Plugin, value::set_value};

/// Plugin property value storage
#[derive(Clone, Debug)]
pub struct Args {
    model: String,
    otlp: bool,
}

impl Default for Args {
    #[inline]
    fn default() -> Self {
        Args {
            model: Self::default_model(),
            otlp: Self::default_otlp(),
        }
    }
}

impl Args {
    #[inline]
    fn default_model() -> String {
        String::default()
    }

    #[inline]
    const fn default_otlp() -> bool {
        false
    }
}

impl ChannelArgs for Args {
    #[inline]
    fn model(&self) -> &String {
        &self.model
    }

    #[inline]
    fn otlp(&self) -> bool {
        self.otlp
    }

    #[inline]
    fn properties() -> &'static [ParamSpec] {
        static PROPERTIES: Params = Params::new(|| Args::default().as_params());

        PROPERTIES.as_ref()
    }

    fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
        match pspec.name() {
            "model" => self.model.to_value(),
            "otlp" => self.otlp.to_value(),
            _ => unimplemented!(),
        }
    }

    fn set_property(
        &mut self,
        plugin: &(impl ?Sized + Plugin),
        _id: usize,
        value: &Value,
        pspec: &ParamSpec,
    ) {
        let name = pspec.name();
        match name {
            "model" => set_value(&plugin.cat(), plugin, name, &mut self.model, value),
            "otlp" => set_value(&plugin.cat(), plugin, name, &mut self.otlp, value),
            _ => unimplemented!(),
        }
    }
}

impl Args {
    fn as_params(&self) -> Vec<ParamSpec> {
        vec![
            ParamSpecString::builder("model")
                .nick("Model")
                .blurb("OpenARK model name")
                .default_value(None)
                .build(),
            ParamSpecBoolean::builder("otlp")
                .nick("OTLP")
                .blurb("Whether to use OpenTelemetry")
                .default_value(self.otlp)
                .build(),
        ]
    }
}

type Params = Lazy<Vec<ParamSpec>>;
