use dash_openapi::image::Image;
use dash_pipe_provider::{messengers::Publisher, PipeMessage};
use gst::{error, glib::subclass::types::ObjectSubclassExt, DebugCategory, FlowError};
use tokio::{sync::mpsc, task::JoinHandle};

use crate::plugin::PluginImpl;

pub(super) struct Queue {
    cat: DebugCategory,
    producer: JoinHandle<()>,
    tx: mpsc::Sender<PipeMessage<Image>>,
}

impl Queue {
    pub(super) async fn try_new<C>(args: super::QueueArgs<'_, C>) -> Result<Self, FlowError>
    where
        C: ?Sized + super::ChannelSubclassExt + PluginImpl,
    {
        let publisher = args
            .call_client(|client, model| async { client.publish(model).await })
            .await?;

        let super::QueueArgs { imp, .. } = args;
        let cat = imp.cat();
        let runtime = imp.runtime();

        let (tx, mut rx) = mpsc::channel(2);
        Ok(Self {
            cat,
            producer: runtime.spawn(async move {
                while let Some(data) = rx.recv().await {
                    if let Err(error) =
                        Publisher::<_, PipeMessage<Image>>::send_one(&publisher, data).await
                    {
                        error!(cat, "Failed to send data: {error}");
                    }
                }
            }),
            tx,
        })
    }

    pub(super) async fn send(
        &self,
        imp: &(impl ?Sized + PluginImpl),
        data: PipeMessage<Image>,
    ) -> Result<(), FlowError> {
        self.tx.send(data).await.map_err(|error| {
            error!(
                self.cat,
                imp: imp,
                "{error}",
            );
            FlowError::Eos
        })
    }

    pub(super) async fn stop(self, imp: &(impl ?Sized + PluginImpl)) {
        let Self { cat, producer, tx } = self;

        producer.abort();

        if let Err(error) = producer.await {
            error!(cat, imp: imp, "Failed to stop sender: {error}");
        }
        drop(tx);
    }
}
