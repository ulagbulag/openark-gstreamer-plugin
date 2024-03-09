use gst::{glib::subclass::types::ObjectSubclass, DebugCategory};
use tokio::{runtime::Runtime, sync::RwLock};

use crate::channel::{Channel, ChannelArgs, ChannelSubclass};

pub trait Plugin
where
    Self: ObjectSubclass,
{
    fn cat(&self) -> DebugCategory;
}

/// Struct containing all the element data
pub struct DynPlugin<Args> {
    args: RwLock<Args>,
    channel: Channel,
    runtime: Runtime,
}

impl<Args> Default for DynPlugin<Args>
where
    Args: Default,
{
    fn default() -> Self {
        let runtime = Runtime::new().expect("Tokio runtime should be created");
        let _guard = runtime.enter();

        Self {
            args: RwLock::default(),
            channel: Channel::default(),
            runtime,
        }
    }
}

impl<Args> ChannelSubclass for DynPlugin<Args>
where
    Args: ChannelArgs,
{
    type Args = Args;

    #[inline]
    fn args(&self) -> &RwLock<Self::Args> {
        &self.args
    }

    #[inline]
    fn channel(&self) -> &Channel {
        &self.channel
    }

    #[inline]
    fn runtime(&self) -> &Runtime {
        &self.runtime
    }
}
