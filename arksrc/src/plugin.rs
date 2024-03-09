use anyhow::Result;
use gsark_common::{
    args::Args,
    channel::{Channel, ChannelSubclass, ChannelSubclassExt},
    plugin::DynPlugin,
};
use gst::{
    glib::{self, subclass::types::ObjectSubclass},
    subclass::prelude::GstObjectImpl,
    BufferRef, DebugCategory, ErrorMessage,
};
use gst_base::subclass::{
    base_src::{BaseSrcImpl, CreateSuccess},
    prelude::PushSrcImpl,
};
use tokio::{runtime::Runtime, sync::RwLock};

/// Struct containing all the element data
#[derive(Default)]
pub struct Plugin(DynPlugin<Args>);

/// This trait registers our type with the GObject object system and
/// provides the entry points for creating a new instance and setting
/// up the class data
#[glib::object_subclass]
impl ObjectSubclass for Plugin {
    const NAME: &'static str = crate::metadata::CLASS_NAME;
    type Type = super::Plugin;
    type ParentType = ::gst_base::PushSrc;
}

impl ::gsark_common::plugin::Plugin for Plugin {
    #[inline]
    fn cat(&self) -> DebugCategory {
        *crate::CAT
    }
}

impl ChannelSubclass for Plugin {
    type Args = Args;

    #[inline]
    fn args(&self) -> &RwLock<<Self as ChannelSubclass>::Args> {
        self.0.args()
    }

    #[inline]
    fn channel(&self) -> &Channel {
        self.0.channel()
    }

    #[inline]
    fn runtime(&self) -> &Runtime {
        self.0.runtime()
    }
}

impl GstObjectImpl for Plugin {}

impl BaseSrcImpl for Plugin {
    #[inline]
    fn start(&self) -> Result<(), ErrorMessage> {
        BaseSrcImpl::unlock_stop(self)?;
        self.runtime().block_on(async {
            <Self as ChannelSubclassExt>::start(self).await?;
            <Self as ChannelSubclassExt>::start_recv(self).await
        })
    }

    #[inline]
    fn stop(&self) -> Result<(), ErrorMessage> {
        BaseSrcImpl::unlock(self)?;
        self.runtime()
            .block_on(<Self as ChannelSubclassExt>::stop(self))
    }

    #[inline]
    fn is_seekable(&self) -> bool {
        false
    }

    #[inline]
    fn size(&self) -> Option<u64> {
        None
    }
}

impl PushSrcImpl for Plugin {
    fn create(&self, buffer: Option<&mut BufferRef>) -> Result<CreateSuccess, gst::FlowError> {
        self.runtime().block_on(self.recv_buffer(buffer))
    }
}
