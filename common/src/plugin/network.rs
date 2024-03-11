use gst::{glib::subclass::types::ObjectSubclass, DebugCategory};
use tokio::{runtime::Runtime, sync::RwLock};

use crate::net::{Channel, ChannelArgs, ChannelSubclass};

use super::base::{ArkSubclass, BasePlugin};

pub trait Plugin
where
    Self: ObjectSubclass,
{
    fn cat(&self) -> DebugCategory;
}

#[derive(Default)]
pub struct NetworkPlugin<Args> {
    base: BasePlugin<Args>,
    channel: Channel,
}

impl<Args> ArkSubclass for NetworkPlugin<Args>
where
    Args: ChannelArgs,
{
    type Args = Args;

    #[inline]
    fn args(&self) -> &RwLock<Self::Args> {
        self.base.args()
    }

    #[inline]
    fn runtime(&self) -> &Runtime {
        self.base.runtime()
    }
}

impl<Args> ChannelSubclass for NetworkPlugin<Args>
where
    Args: ChannelArgs,
{
    #[inline]
    fn channel(&self) -> &Channel {
        &self.channel
    }
}
