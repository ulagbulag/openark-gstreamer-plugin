use gsark_common::value::set_value;
use gst::glib::{
    self, subclass::object::ObjectImpl, value::ToValue, ParamSpec, ParamSpecBuilderExt,
};
use once_cell::sync::Lazy;

/// Plugin property value storage
#[derive(Clone, Debug)]
pub struct Args {
    model: String,
    otlp: bool,
}

impl Default for Args {
    fn default() -> Self {
        Args {
            model: Self::default_model(),
            otlp: Self::default_otlp(),
        }
    }
}

impl Args {
    fn default_model() -> String {
        String::default()
    }

    const fn default_otlp() -> bool {
        false
    }
}

impl Args {
    pub fn model(&self) -> &String {
        &self.model
    }

    pub const fn otlp(&self) -> bool {
        self.otlp
    }
}

impl Args {
    fn properties() -> &'static [glib::ParamSpec] {
        static PROPERTIES: Params = Params::new(|| Args::default().as_params());

        PROPERTIES.as_ref()
    }

    fn as_params(&self) -> Vec<ParamSpec> {
        vec![
            glib::ParamSpecString::builder("model")
                .nick("Model")
                .blurb("OpenARK model name")
                .default_value(None)
                .build(),
            glib::ParamSpecBoolean::builder("otlp")
                .nick("OTLP")
                .blurb("Whether to use OpenTelemetry")
                .default_value(self.otlp)
                .build(),
        ]
    }
}

/// Implementation of glib::Object virtual methods
impl ObjectImpl for crate::plugin::Plugin {
    fn properties() -> &'static [glib::ParamSpec] {
        Args::properties()
    }

    /// Called whenever a value of a property is changed. It can be called
    /// at any time from any thread.
    fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
        let mut args = self.args.blocking_write();

        let name = pspec.name();
        match name {
            "model" => set_value(&crate::CAT, self, name, &mut args.model, value),
            "otlp" => set_value(&crate::CAT, self, name, &mut args.otlp, value),
            _ => unimplemented!(),
        }
    }

    /// Called whenever a value of a property is read. It can be called
    /// at any time from any thread.
    fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        let settings = self.args.blocking_read();
        match pspec.name() {
            "model" => settings.model.to_value(),
            "otlp" => settings.otlp.to_value(),
            _ => unimplemented!(),
        }
    }
}

type Params = Lazy<Vec<glib::ParamSpec>>;
