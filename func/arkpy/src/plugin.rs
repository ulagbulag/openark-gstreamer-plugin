use anyhow::Result;
use bytes::Bytes;
use dash_pipe_function_python_provider::Function;
use dash_pipe_provider::{DynValue, PipeMessage, PipeMessages, PipePayload, RemoteFunction};
use gsark_common::plugin::{
    base::{ArkSubclass, BasePlugin},
    PluginImpl,
};
use gst::{
    debug, error,
    glib::{
        self,
        subclass::types::{ObjectSubclass, ObjectSubclassExt},
    },
    subclass::prelude::GstObjectImpl,
    Buffer, BufferRef, Caps, DebugCategory, ErrorMessage, FlowError, FlowSuccess,
};
use gst_base::subclass::{base_transform::BaseTransformImpl, BaseTransformMode};
use tokio::{runtime::Runtime, sync::RwLock};

use crate::args::Args;

/// Struct containing all the element data
#[derive(Default)]
pub struct Plugin {
    base: BasePlugin<Args>,
    function: RwLock<Option<Box<dyn RemoteFunction>>>,
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

    fn start(&self) -> Result<(), ErrorMessage> {
        Ok(())
    }

    fn stop(&self) -> Result<(), ErrorMessage> {
        Ok(())
    }

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
}

impl Plugin {
    #[inline]
    fn call_function(&self, input: &[u8]) -> Result<impl AsRef<[u8]>, FlowError> {
        self.runtime().block_on(self.call_function_async(input))
    }

    async fn call_function_async(&self, input: &[u8]) -> Result<impl AsRef<[u8]>, FlowError> {
        let inputs = PipeMessages::Single(self.decode(input)?);
        let outputs = inputs;
        let output = match outputs {
            PipeMessages::Single(output) => output,
            _ => {
                error!(
                    crate::CAT,
                    imp: self,
                    "Unexpected output pipe message type",
                );
                return Err(FlowError::Error);
            }
        };

        // let output: PipeMessage = PipeMessage::new(DynValue::Null);
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
        // TODO: to be implemented
        Ok(message.payloads.pop().unwrap().value().cloned().unwrap())
    }
}
