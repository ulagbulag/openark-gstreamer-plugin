use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::Result;
use bytes::Bytes;
use dash_openapi::image::Image;
use dash_pipe_provider::{PipeMessage, PipePayload};
use gsark_common::{
    args::Args,
    channel::{Channel, ChannelSubclass, ChannelSubclassExt},
    plugin::DynPlugin,
};
use gst::{
    error,
    glib::{
        self,
        subclass::types::{ObjectSubclass, ObjectSubclassExt},
    },
    subclass::prelude::GstObjectImpl,
    Buffer, DebugCategory, ErrorMessage, FlowError, FlowSuccess,
};
use gst_base::subclass::prelude::BaseSinkImpl;
use tokio::{runtime::Runtime, sync::RwLock};

/// Struct containing all the element data
#[derive(Default)]
pub struct Plugin {
    counter: AtomicUsize,
    inner: DynPlugin<Args>,
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
        self.inner.args()
    }

    #[inline]
    fn channel(&self) -> &Channel {
        self.inner.channel()
    }

    #[inline]
    fn runtime(&self) -> &Runtime {
        self.inner.runtime()
    }
}

impl GstObjectImpl for Plugin {}

impl BaseSinkImpl for Plugin {
    fn start(&self) -> Result<(), ErrorMessage> {
        BaseSinkImpl::unlock_stop(self)?;
        self.runtime()
            .block_on(<Self as ChannelSubclassExt>::start(self))
    }

    fn stop(&self) -> Result<(), ErrorMessage> {
        BaseSinkImpl::unlock(self)?;
        self.runtime()
            .block_on(<Self as ChannelSubclassExt>::stop(self))
    }

    fn render(&self, buffer: &Buffer) -> Result<FlowSuccess, FlowError> {
        self.runtime().block_on(async {
            // get data index
            let index = self.counter.fetch_add(1, Ordering::SeqCst);

            // TODO: support non-image(video) data using sink Caps and cache it
            // parse data extension
            let ext = ".jpg";

            // TODO: support non-image(video) data using sink Caps and cache it
            // build a payload
            let key = format!("{index:06}{ext}");
            let key_ref = format!("@data:image,{key}");
            let payload = PipePayload::new(
                key,
                Some(Bytes::from(buffer.map_readable().unwrap().to_vec())),
            );

            // build a message
            // TODO: to be implemented
            let value = Image::default();
            let message = PipeMessage::with_payloads(vec![payload], value);

            // encode and send
            self.send(message)
                .await
                .map(|()| FlowSuccess::Ok)
                .map_err(|error| {
                    error!(
                        crate::CAT,
                        imp: self,
                        "{error}",
                    );
                    FlowError::Error
                })
        })
    }
}
