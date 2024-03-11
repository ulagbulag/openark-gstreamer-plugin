use tokio::{runtime::Runtime, sync::RwLock};

use crate::net::ChannelArgs;

pub struct BasePlugin<Args> {
    args: RwLock<Args>,
    runtime: Runtime,
}

impl<Args> Default for BasePlugin<Args>
where
    Args: Default,
{
    fn default() -> Self {
        let runtime = Runtime::new().expect("Tokio runtime should be created");
        let _guard = runtime.enter();

        Self {
            args: RwLock::default(),
            runtime,
        }
    }
}

pub trait ArkSubclass {
    type Args: ChannelArgs;

    fn args(&self) -> &RwLock<<Self as ArkSubclass>::Args>;

    fn runtime(&self) -> &Runtime;
}

impl<Args> ArkSubclass for BasePlugin<Args>
where
    Args: ChannelArgs,
{
    type Args = Args;

    #[inline]
    fn args(&self) -> &RwLock<Self::Args> {
        &self.args
    }

    #[inline]
    fn runtime(&self) -> &Runtime {
        &self.runtime
    }
}
