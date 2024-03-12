use std::path::PathBuf;

use dash_pipe_function_python_provider::FunctionArgs;
use gsark_common::{
    args::Params,
    net::ChannelArgs,
    plugin::{base::ArkSubclass, PluginImpl},
    value::set_value,
};
use gst::glib::{
    subclass::object::ObjectImpl, value::ToValue, ParamSpec, ParamSpecBuilderExt, ParamSpecString,
    Value,
};

/// Plugin property value storage
#[derive(Clone, Debug)]
pub struct Args {
    common: ::gsark_common::args::Args,
    file: Option<PathBuf>,
    method: String,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            common: Default::default(),
            file: Default::default(),
            method:
                ::dash_pipe_function_python_provider::FunctionArgs::default_python_tick_method_str()
                    .into(),
        }
    }
}

impl ChannelArgs for Args {
    #[inline]
    fn model(&self) -> &String {
        self.common.model()
    }

    #[inline]
    fn otlp(&self) -> bool {
        self.common.otlp()
    }

    #[inline]
    fn properties() -> &'static [ParamSpec] {
        static PROPERTIES: Params = Params::new(|| Args::default().as_params());

        PROPERTIES.as_ref()
    }

    #[inline]
    fn property(&self, id: usize, pspec: &ParamSpec) -> Value {
        match pspec.name() {
            "file" => self.file.to_value(),
            "method" => self.method.to_value(),
            _ => self.common.property(id, pspec),
        }
    }

    #[inline]
    fn set_property(
        &mut self,
        plugin: &(impl ?Sized + PluginImpl),
        id: usize,
        value: &Value,
        pspec: &ParamSpec,
    ) {
        let name = pspec.name();
        match name {
            "file" => set_value(plugin, name, &mut self.file, value),
            "method" => set_value(plugin, name, &mut self.method, value),
            _ => self.common.set_property(plugin, id, value, pspec),
        }
    }
}

/// Implementation of glib::Object virtual methods
impl ObjectImpl for crate::plugin::Plugin {
    #[inline]
    fn properties() -> &'static [ParamSpec] {
        Args::properties()
    }

    #[inline]
    fn set_property(&self, id: usize, value: &Value, pspec: &ParamSpec) {
        self.args()
            .blocking_write()
            .set_property(self, id, value, pspec)
    }

    #[inline]
    fn property(&self, id: usize, pspec: &ParamSpec) -> Value {
        self.args().blocking_read().property(id, pspec)
    }
}

impl Args {
    fn as_params(&self) -> Vec<ParamSpec> {
        let mut params = self.common.as_params();
        params.push(
            ParamSpecString::builder("file")
                .nick("Pythonfile")
                .blurb("Python script file path")
                .build(),
        );
        params.push(
            ParamSpecString::builder("method")
                .nick("PyMethod")
                .blurb("Python `tick` method name in the script")
                .default_value(Some(self.method.as_str()))
                .build(),
        );
        params
    }

    pub fn build(&self) -> Option<FunctionArgs> {
        Some(FunctionArgs {
            python_script: self.file.clone()?,
            python_tick_method: self.method.clone(),
        })
    }
}
