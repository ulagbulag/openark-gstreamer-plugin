use gsark_common::{args::Args, net::ChannelArgs, plugin::base::ArkSubclass};
use gst::glib::{subclass::object::ObjectImpl, ParamSpec, Value};

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
