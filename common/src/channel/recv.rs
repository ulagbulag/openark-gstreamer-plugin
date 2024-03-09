use std::time::Duration;

use bytes::Bytes;
use dash_pipe_provider::messengers::Subscriber;
use gst::{error, glib::subclass::types::ObjectSubclassExt, DebugCategory, FlowError};
use tokio::{
    sync::mpsc::{self, error::SendTimeoutError},
    task::JoinHandle,
};

use crate::plugin::Plugin;

pub(super) struct Queue {
    cat: DebugCategory,
    producer: JoinHandle<()>,
    rx: mpsc::Receiver<Bytes>,
}

impl Queue {
    pub(super) async fn try_new<C>(args: super::QueueArgs<'_, C>) -> Result<Self, FlowError>
    where
        C: ?Sized + super::ChannelSubclass + Plugin,
    {
        let mut subscriber = args
            .call_client(|client, model| async { client.subscribe(model).await })
            .await?;

        let super::QueueArgs { imp, .. } = args;
        let cat = imp.cat();
        let runtime = imp.runtime();

        let (tx, rx) = mpsc::channel(4);
        Ok(Self {
            cat,
            producer: runtime.spawn(async move {
                loop {
                    match subscriber.read_one().await {
                        Ok(Some(mut msg)) => {
                            if let Some(data) = msg
                                .payloads
                                .pop()
                                .and_then(|payload| payload.value().cloned())
                            {
                                match tx.send_timeout(data, Duration::from_millis(10)).await {
                                    Ok(()) | Err(SendTimeoutError::Timeout(_)) => continue,
                                    // Queue is destroying, stop sending.
                                    Err(SendTimeoutError::Closed(_)) => break,
                                }
                            }
                        }
                        // Subscriber is destroying, stop sending.
                        Ok(None) => break,
                        Err(error) => {
                            error!(cat, "Failed to receive data: {error}");
                        }
                    }
                }
            }),
            rx,
        })
    }

    #[inline]
    pub(super) async fn recv(&mut self) -> Option<Bytes> {
        self.rx.recv().await
    }

    pub(super) async fn stop(self, imp: &(impl ?Sized + Plugin)) {
        let Self {
            cat,
            producer,
            mut rx,
        } = self;

        rx.close();
        producer.abort();

        if let Err(error) = producer.await {
            error!(cat, imp: imp, "Failed to stop receiver: {error}");
        }
    }
}
