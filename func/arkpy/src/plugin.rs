use std::sync::Arc;

use anyhow::Result;
use bytes::Bytes;
use dash_pipe_function_python_provider::Function;
use dash_pipe_provider::{
    storage::StorageIO, Codec, DynValue, FunctionBuilder, PipeClient, PipeMessage, PipePayload,
    RemoteFunction,
};
use gsark_common::{
    net::try_init_client,
    plugin::{
        base::{ArkSubclass, BasePlugin},
        PluginImpl,
    },
    sync,
};
use gst::{
    debug, error, error_msg,
    glib::{
        self, gstr,
        subclass::types::{ObjectSubclass, ObjectSubclassExt},
    },
    subclass::prelude::GstObjectImpl,
    Buffer, BufferRef, Caps, CapsIntersectMode, CoreError, DebugCategory, ErrorMessage, FlowError,
    FlowSuccess, PadDirection,
};
use gst_base::subclass::{base_transform::BaseTransformImpl, BaseTransformMode};
use tokio::{
    runtime::Runtime,
    sync::{MappedMutexGuard, Mutex, RwLock},
};

use crate::args::Args;

/// Struct containing all the element data
#[derive(Default)]
pub struct Plugin {
    base: BasePlugin<Args>,
    client: Mutex<Option<PipeClient>>,
    function: Mutex<Option<DynFunction>>,
}

/// This trait registers our type with the GObject object system and
/// provides the entry points for creating a new instance and setting
/// up the class data
#[glib::object_subclass]
impl ObjectSubclass for Plugin {
    const NAME: &'static str = crate::metadata::CLASS_NAME;
    type Type = super::Plugin;
    type ParentType = ::gst_base::BaseTransform;
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
        self.base.args()
    }

    #[inline]
    fn runtime(&self) -> &Runtime {
        self.base.runtime()
    }
}

impl GstObjectImpl for Plugin {}

impl BaseTransformImpl for Plugin {
    const MODE: BaseTransformMode = BaseTransformMode::NeverInPlace;
    const PASSTHROUGH_ON_SAME_CAPS: bool = false;
    const TRANSFORM_IP_ON_PASSTHROUGH: bool = false;

    #[inline]
    fn start(&self) -> Result<(), ErrorMessage> {
        self.runtime().block_on(self.init_function()).map(|_| ())
    }

    #[inline]
    fn stop(&self) -> Result<(), ErrorMessage> {
        self.runtime().block_on(self.stop_function());
        Ok(())
    }

    #[inline]
    fn unit_size(&self, _caps: &Caps) -> Option<usize> {
        Some(1)
    }

    fn transform(&self, inbuf: &Buffer, outbuf: &mut BufferRef) -> Result<FlowSuccess, FlowError> {
        // execute function
        let output = self.call_function(&inbuf.map_readable().unwrap())?;
        let output = output.as_ref();

        // write to buffer
        outbuf.set_size(output.len());
        let mut buffer = outbuf.map_writable().unwrap();
        buffer.copy_from_slice(output);

        debug!(
            crate::CAT,
            imp: self,
            "Passed function",
        );

        Ok(FlowSuccess::Ok)
    }

    fn transform_caps(
        &self,
        direction: PadDirection,
        caps: &Caps,
        filter: Option<&Caps>,
    ) -> Option<Caps> {
        let other_caps = match direction {
            // src -> sink
            PadDirection::Sink => {
                let function = self.runtime().block_on(self.init_function());

                Caps::builder(gstr!("application/x-json")).build()
            }
            // sink -> src
            PadDirection::Src => {
                let function = self.runtime().block_on(self.init_function());

                Caps::builder(gstr!("video/x-raw")).build()
            }
            PadDirection::Unknown => caps.clone(),
        };

        debug!(
            crate::CAT,
            imp: self,
            "Transformed caps from {} to {} in direction {:?}",
            caps,
            other_caps,
            direction,
        );

        filter
            .map(|filter| filter.intersect_with_mode(&other_caps, CapsIntersectMode::First))
            .or(Some(other_caps))
    }
}

impl Plugin {
    async fn init_client(&self) -> Result<MappedMutexGuard<'_, PipeClient>, ErrorMessage> {
        let mut lock = self.client.lock().await;
        match lock.as_ref() {
            Some(_) => Ok(sync::mutex::unwrap_lock(lock)),
            None => {
                lock.replace(try_init_client().await?);
                Ok(sync::mutex::unwrap_lock(lock))
            }
        }
    }

    async fn init_function(&self) -> Result<MappedMutexGuard<'_, DynFunction>, ErrorMessage> {
        let mut lock = self.function.lock().await;
        match lock.as_ref() {
            Some(_) => Ok(sync::mutex::unwrap_lock(lock)),
            None => {
                let args = {
                    let lock = self.args().read().await;
                    lock.build().ok_or_else(|| {
                        error_msg!(
                            CoreError::Failed,
                            ["Failed to parse OpenARK function parameters"]
                        )
                    })?
                };

                let storage = {
                    let client = self.init_client().await?;
                    Arc::new(StorageIO {
                        input: client.storage().clone(),
                        output: client.storage().clone(),
                    })
                };

                let function = Function::try_new(&args, None, &storage)
                    .await
                    .map_err(|error| {
                        error_msg!(
                            CoreError::Failed,
                            ["Failed to init OpenARK function: {error}"]
                        )
                    })?;

                lock.replace(Box::new(function));
                Ok(sync::mutex::unwrap_lock(lock))
            }
        }
    }

    #[inline]
    fn call_function(&self, input: &[u8]) -> Result<impl AsRef<[u8]>, FlowError> {
        self.runtime().block_on(self.call_function_async(input))
    }

    async fn call_function_async(&self, input: &[u8]) -> Result<impl AsRef<[u8]>, FlowError> {
        let input = self.decode(input)?;

        let function_lock = self.function.lock().await;
        let function = function_lock.as_ref().ok_or_else(|| {
            error!(
                crate::CAT,
                imp: self,
                "OpenARK function is not inited!",
            );
            FlowError::Error
        })?;

        let output = function.call_one(input).await.map_err(|error| {
            error!(
                crate::CAT,
                imp: self,
                "Failed to execute OpenARK function: {error}",
            );
            FlowError::Error
        })?;
        drop(function_lock);

        self.encode(output)
    }

    fn decode(&self, data: &[u8]) -> Result<PipeMessage, FlowError> {
        // TODO: handle other media types (audio, JSON, plain, ...)
        let key = "image".into();
        let payload = PipePayload::new(key, Some(Bytes::from(data.to_vec())));

        let input = PipeMessage::with_payloads(vec![payload], DynValue::Null);
        Ok(input)
    }

    fn encode(&self, mut message: PipeMessage) -> Result<Bytes, FlowError> {
        // TODO: handle other media types (audio, JSON, plain, ...)
        match message.payloads.len() {
            0 => message.to_bytes(Codec::Json).map_err(|error| {
                error!(
                    crate::CAT,
                    imp: self,
                    "Failed to encode OpenARK function output: {error}",
                );
                FlowError::Error
            }),
            1 => Ok(message.payloads.pop().unwrap().value().cloned().unwrap()),
            2.. => todo!(),
        }
    }

    async fn stop_function(&self) {
        self.function.lock().await.take();
    }
}

type DynFunction = Box<dyn RemoteFunction<Input = DynValue, Output = DynValue>>;
