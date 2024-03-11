use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::Result;
use gsark_common::{
    args::Args,
    net::{Channel, ChannelSubclass, ChannelSubclassExt},
    plugin::{base::ArkSubclass, network::NetworkPlugin, PluginImpl},
};
use gst::{
    glib::{self, subclass::types::ObjectSubclass},
    subclass::prelude::GstObjectImpl,
    Buffer, DebugCategory, ErrorMessage, FlowError, FlowSuccess,
};
use gst_base::subclass::prelude::BaseSinkImpl;
use tokio::{runtime::Runtime, sync::RwLock};

/// Struct containing all the element data
#[derive(Default)]
pub struct Plugin {
    counter: AtomicU64,
    network: NetworkPlugin<Args>,
}

/// This trait registers our type with the GObject object system and
/// provides the entry points for creating a new instance and setting
/// up the class data
#[glib::object_subclass]
impl ObjectSubclass for Plugin {
    const NAME: &'static str = crate::metadata::CLASS_NAME;
    type Type = super::Plugin;
    type ParentType = ::gst_base::BaseSink;
}

impl PluginImpl for Plugin {
    #[inline]
    fn cat(&self) -> DebugCategory {
        *crate::CAT
    }
}

impl ArkSubclass for Plugin {
    type Args = Args;

    #[inline]
    fn args(&self) -> &RwLock<<Self as ArkSubclass>::Args> {
        self.network.args()
    }

    #[inline]
    fn runtime(&self) -> &Runtime {
        self.network.runtime()
    }
}

impl ChannelSubclass for Plugin {
    #[inline]
    fn channel(&self) -> &Channel {
        self.network.channel()
    }
}

impl GstObjectImpl for Plugin {}

impl BaseSinkImpl for Plugin {
    fn start(&self) -> Result<(), ErrorMessage> {
        BaseSinkImpl::unlock_stop(self)?;
        self.runtime().block_on(async {
            <Self as ChannelSubclassExt>::start(self).await?;
            <Self as ChannelSubclassExt>::start_send(self).await
        })
    }

    fn stop(&self) -> Result<(), ErrorMessage> {
        BaseSinkImpl::unlock(self)?;
        self.runtime()
            .block_on(<Self as ChannelSubclassExt>::stop(self))
    }

    fn render(&self, buffer: &Buffer) -> Result<FlowSuccess, FlowError> {
        // get data index
        let index = self.counter.fetch_add(1, Ordering::SeqCst);

        // TODO: support non-image(video) data using sink Caps and cache it
        // parse data extension
        let ext = ".jpg";

        // TODO: support non-image(video) data using sink Caps and cache it
        // build a payload
        let key = format!("{index:06}{ext}");

        self.runtime().block_on(self.send_buffer(key, buffer))
    }
}
