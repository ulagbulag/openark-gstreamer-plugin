use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{anyhow, Result};
use bytes::Bytes;
use dash_pipe_provider::messengers::Publisher;
use dash_pipe_provider::{PipeMessage, PipePayload};
use gsark_common::client;
use gst::{
    glib::{
        self,
        subclass::types::{ObjectSubclass, ObjectSubclassExt},
    },
    subclass::prelude::GstObjectImpl,
    Buffer, CoreError, ErrorMessage, FlowError, FlowSuccess,
};
use gst_base::subclass::prelude::BaseSinkImpl;
use serde_json::json;
use serde_json::Value;
use tokio::{
    runtime::Runtime,
    sync::{mpsc, RwLock},
    task::JoinHandle,
};

use crate::args::Args;

/// Struct containing all the element data
pub struct Plugin {
    pub(crate) args: RwLock<Args>,
    counter: AtomicUsize,
    queue: RwLock<Option<Queue>>,
    runtime: Runtime,
}

impl Default for Plugin {
    fn default() -> Self {
        let runtime = Runtime::new().expect("Tokio runtime should be created");
        let _guard = runtime.enter();

        Self {
            args: RwLock::default(),
            counter: AtomicUsize::default(),
            queue: RwLock::default(),
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
    type ParentType = ::gst_base::BaseSink;
}

impl GstObjectImpl for Plugin {}

impl BaseSinkImpl for Plugin {
    fn start(&self) -> Result<(), ErrorMessage> {
        BaseSinkImpl::unlock_stop(self)?;
        self.runtime.block_on(self.start_sender())?;

        gst::info!(
            crate::CAT,
            imp: self,
            "Started",
        );
        Ok(())
    }

    fn stop(&self) -> Result<(), ErrorMessage> {
        BaseSinkImpl::unlock(self)?;
        self.runtime.block_on(self.stop_sender())?;

        gst::info!(
            crate::CAT,
            imp: self,
            "Stopped",
        );
        Ok(())
    }

    fn render(&self, buffer: &Buffer) -> Result<FlowSuccess, FlowError> {
        self.runtime.block_on(async {
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
            let value = json! ({
                "image": key_ref,
            });
            let message = PipeMessage::with_payloads(vec![payload], value);

            // encode and send
            self.send(message)
                .await
                .map(|()| FlowSuccess::Ok)
                .map_err(|error| {
                    gst::error!(
                        crate::CAT,
                        imp: self,
                        "{error}",
                    );
                    FlowError::Error
                })
        })
    }
}

impl Plugin {
    async fn start_sender(&self) -> Result<(), ErrorMessage> {
        let args = self.args.read().await;
        let model = args.model().clone();
        let otlp = args.otlp();
        drop(args);

        let mut queue = self.queue.write().await;
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

    async fn stop_sender(&self) -> Result<(), ErrorMessage> {
        let mut queue = self.queue.write().await;
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

    async fn send(&self, data: PipeMessage<Value>) -> Result<(), FlowError> {
        self.queue
            .read()
            .await
            .as_ref()
            .ok_or(FlowError::Eos)?
            .send(data)
            .await
            .map_err(|error| {
                gst::error!(
                    crate::CAT,
                    imp: self,
                    "{error}",
                );
                FlowError::Eos
            })
    }
}

struct Queue {
    producer: JoinHandle<Result<()>>,
    tx: mpsc::Sender<PipeMessage<Value>>,
}

impl Queue {
    async fn try_new(runtime: &Runtime, model: String, otlp: bool) -> Result<Self> {
        let client = client::try_init(otlp).await?;
        let publisher = client.publish(model.parse()?).await?;

        let (tx, mut rx) = mpsc::channel(2);
        Ok(Self {
            producer: runtime.spawn(async move {
                while let Some(data) = rx.recv().await {
                    if let Err(error) =
                        Publisher::<PipeMessage<Value>, PipeMessage<Value>>::send_one(
                            &publisher, data,
                        )
                        .await
                    {
                        gst::error!(crate::CAT, "Failed to send data: {error}",);
                    }
                }
                Ok(())
            }),
            tx,
        })
    }

    async fn send(&self, data: PipeMessage<Value>) -> Result<()> {
        self.tx
            .send(data)
            .await
            .map_err(|error| anyhow!("Failed to send data: {error}"))
    }

    async fn stop(self) -> Result<()> {
        self.producer.abort();
        self.producer.await.unwrap_or(Ok(()))
    }
}
