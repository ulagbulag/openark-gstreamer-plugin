use std::time::Duration;

use anyhow::Result;
use bytes::Bytes;
use gsark_common::client;
use gst::{
    glib::{
        self,
        subclass::types::{ObjectSubclass, ObjectSubclassExt},
    },
    subclass::prelude::GstObjectImpl,
    Buffer, CoreError, ErrorMessage, FlowError,
};
use gst_base::subclass::{
    base_src::{BaseSrcImpl, CreateSuccess},
    prelude::PushSrcImpl,
};
use tokio::{
    runtime::Runtime,
    sync::{
        mpsc::{self, error::SendTimeoutError},
        Mutex, RwLock,
    },
    task::JoinHandle,
};

use crate::args::Args;

/// Struct containing all the element data
pub struct Plugin {
    pub(crate) args: RwLock<Args>,
    queue: Mutex<Option<Queue>>,
    runtime: Runtime,
}

impl Default for Plugin {
    fn default() -> Self {
        let runtime = Runtime::new().expect("Tokio runtime should be created");
        let _guard = runtime.enter();

        Self {
            args: RwLock::default(),
            queue: Mutex::default(),
            runtime,
        }
    }
}

/// This trait registers our type with the GObject object system and
/// provides the entry points for creating a new instance and setting
/// up the class data
#[glib::object_subclass]
impl ObjectSubclass for Plugin {
    const NAME: &'static str = crate::metadata::CLASS_NAME;
    type Type = super::Plugin;
    type ParentType = ::gst_base::PushSrc;
}

impl GstObjectImpl for Plugin {}

impl BaseSrcImpl for Plugin {
    fn start(&self) -> Result<(), ErrorMessage> {
        BaseSrcImpl::unlock_stop(self)?;
        self.runtime.block_on(self.start_receiver())?;

        gst::info!(
            crate::CAT,
            imp: self,
            "Started",
        );
        Ok(())
    }

    fn stop(&self) -> Result<(), ErrorMessage> {
        BaseSrcImpl::unlock(self)?;
        self.runtime.block_on(self.stop_receiver())?;

        gst::info!(
            crate::CAT,
            imp: self,
            "Stopped",
        );
        Ok(())
    }

    fn is_seekable(&self) -> bool {
        false
    }

    fn size(&self) -> Option<u64> {
        None
    }
}

impl PushSrcImpl for Plugin {
    fn create(
        &self,
        buffer: Option<&mut gst::BufferRef>,
    ) -> Result<gst_base::subclass::base_src::CreateSuccess, gst::FlowError> {
        self.runtime.block_on(async {
            // load a message
            let message = match self.next().await {
                Some(message) => message,
                None => return Err(FlowError::Eos),
            };

            // TODO: is buffer used?
            if buffer.is_some() {
                todo!();
            }

            // create a stream buffer
            let buffer = Buffer::from_slice(message);

            gst::debug!(
                crate::CAT,
                imp: self,
                "Produced buffer {buffer:?}",
            );

            Ok(CreateSuccess::NewBuffer(buffer))
        })
    }
}

impl Plugin {
    async fn start_receiver(&self) -> Result<(), ErrorMessage> {
        let args = self.args.read().await;
        let model = args.model().clone();
        let otlp = args.otlp();
        drop(args);

        let mut queue = self.queue.lock().await;
        if queue.is_none() {
            match Queue::try_new(&self.runtime, model, otlp).await {
                Ok(q) => {
                    queue.replace(q);
                    Ok(())
                }
                Err(error) => Err(gst::error_msg!(
                    CoreError::Failed,
                    ["Failed to init OpenARK client: {error}"]
                )),
            }
        } else {
            Ok(())
        }
    }

    async fn stop_receiver(&self) -> Result<(), ErrorMessage> {
        let mut queue = self.queue.lock().await;
        if let Some(result) = queue.take() {
            result.stop().await.map_err(|error| {
                gst::error_msg!(
                    CoreError::Failed,
                    ["Failed to deinit OpenARK client: {error}"]
                )
            })
        } else {
            Ok(())
        }
    }

    async fn next(&self) -> Option<Bytes> {
        self.queue.lock().await.as_mut()?.next().await
    }
}

struct Queue {
    producer: JoinHandle<Result<()>>,
    rx: mpsc::Receiver<Bytes>,
}

impl Queue {
    async fn try_new(runtime: &Runtime, model: String, otlp: bool) -> Result<Self> {
        let client = client::try_init(otlp).await?;
        let mut subscriber = client.subscribe(model.parse()?).await?;

        let (tx, rx) = mpsc::channel(4);
        Ok(Self {
            producer: runtime.spawn(async move {
                loop {
                    if let Some(data) = subscriber
                        .read_one()
                        .await?
                        .and_then(|mut msg| msg.payloads.pop())
                        .and_then(|payload| payload.value().cloned())
                    {
                        if tx
                            .send_timeout(data, Duration::from_millis(10))
                            .await
                            .or_else(|error| match error {
                                SendTimeoutError::Timeout(_) => Ok(()),
                                _ => Err(error),
                            })
                            .is_err()
                        {
                            // Queue is destroying, stop sending.
                            break Ok(());
                        }
                    }
                }
            }),
            rx,
        })
    }

    async fn next(&mut self) -> Option<Bytes> {
        self.rx.recv().await
    }

    async fn stop(mut self) -> Result<()> {
        self.rx.close();
        self.producer
            .await
            .map_err(Into::into)
            .and_then(::core::convert::identity)
    }
}
