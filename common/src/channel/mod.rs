mod recv;
mod send;

use std::future::Future;

use anyhow::Result;
use ark_core::tracer;
use async_trait::async_trait;
use bytes::Bytes;
use clap::Parser;
use dash_openapi::image::Image;
use dash_pipe_provider::{Name, PipeClient, PipeClientArgs, PipeMessage};
use gst::{
    error, error_msg,
    glib::{subclass::types::ObjectSubclassExt, ParamSpec, Value},
    info, CoreError, ErrorMessage, FlowError,
};
use tokio::{
    join,
    runtime::Runtime,
    sync::{Mutex, RwLock, RwLockReadGuard},
};

use crate::plugin::Plugin;

pub trait ChannelArgs
where
    Self: Send + Sync + Default,
{
    fn model(&self) -> &String;

    fn otlp(&self) -> bool;

    /// Properties installed for this type.
    fn properties() -> &'static [ParamSpec];

    /// Called whenever a value of a property is read. It can be called
    /// at any time from any thread.
    fn property(&self, id: usize, pspec: &ParamSpec) -> Value;

    /// Called whenever a value of a property is changed. It can be called
    /// at any time from any thread.
    fn set_property(
        &mut self,
        plugin: &(impl ?Sized + Plugin),
        id: usize,
        value: &Value,
        pspec: &ParamSpec,
    );
}

pub trait ChannelSubclass {
    type Args: ChannelArgs;

    fn args(&self) -> &RwLock<<Self as ChannelSubclass>::Args>;

    fn channel(&self) -> &Channel;

    fn runtime(&self) -> &Runtime;
}

#[async_trait]
pub trait ChannelSubclassExt
where
    Self: ChannelSubclass + Plugin,
{
    async fn start(&self) -> Result<(), ErrorMessage> {
        let args = self.args().read().await;
        let model = args.model().clone();
        let otlp = args.otlp();
        drop(args);

        ChannelBuilder::new(model)
            .otlp(otlp)
            .build(self.channel())
            .await?;

        info!(
            self.cat(),
            imp: self,
            "Started",
        );
        Ok(())
    }

    #[inline]
    async fn recv(&self) -> Result<Option<Bytes>, FlowError> {
        self.channel().recv(self).await
    }

    #[inline]
    async fn send(&self, data: PipeMessage<Image>) -> Result<(), FlowError> {
        self.channel().send(self, data).await
    }

    #[inline]
    async fn stop(&self) -> Result<(), ErrorMessage> {
        self.channel().stop(self).await;

        info!(
            self.cat(),
            imp: self,
            "Stopped",
        );
        Ok(())
    }
}

#[async_trait]
impl<T> ChannelSubclassExt for T where Self: ChannelSubclass + Plugin {}

#[derive(Default)]
pub struct Channel {
    builder: RwLock<Option<ChannelBuilder>>,
    client: RwLock<Option<PipeClient<Image>>>,
    recv: Mutex<Option<self::recv::Queue>>,
    send: RwLock<Option<self::send::Queue>>,
}

impl Channel {
    async fn recv(
        &self,
        imp: &(impl ?Sized + ChannelSubclass + Plugin),
    ) -> Result<Option<Bytes>, FlowError> {
        let mut lock = self.recv.lock().await;
        let queue = match lock.as_mut() {
            Some(queue) => queue,
            None => {
                let builder_lock = self.builder.read().await;
                match builder_lock.as_ref() {
                    Some(builder) => {
                        let client_lock = self.client.read().await;
                        let client = assert_client(&client_lock, imp)?;

                        lock.replace(builder.build_receiver(client, imp).await?);
                        drop(client_lock);
                        drop(builder_lock);

                        lock.as_mut().unwrap()
                    }
                    None => return Ok(None),
                }
            }
        };
        Ok(queue.recv().await)
    }

    async fn send(
        &self,
        imp: &(impl ?Sized + ChannelSubclass + Plugin),
        data: PipeMessage<Image>,
    ) -> Result<(), FlowError> {
        let lock = self.send.read().await;
        match lock.as_ref() {
            Some(queue) => queue.send(imp, data).await,
            None => {
                drop(lock);

                let builder_lock = self.builder.read().await;
                match builder_lock.as_ref() {
                    Some(builder) => {
                        let client_lock = self.client.read().await;
                        let client = assert_client(&client_lock, imp)?;

                        let mut lock = self.send.write().await;
                        lock.replace(builder.build_sender(client, imp).await?);
                        drop(client_lock);
                        drop(builder_lock);

                        let queue = lock.as_ref().unwrap();
                        queue.send(imp, data).await
                    }
                    None => Err(FlowError::Eos),
                }
            }
        }
    }

    async fn stop(&self, imp: &(impl ?Sized + ChannelSubclass + Plugin)) {
        let stop_recv = async {
            let maybe_queue = {
                let mut lock = self.recv.lock().await;
                lock.take()
            };
            if let Some(queue) = maybe_queue {
                queue.stop(imp).await
            }
        };

        let stop_send = async {
            let maybe_queue = {
                let mut lock = self.send.write().await;
                lock.take()
            };
            if let Some(queue) = maybe_queue {
                queue.stop(imp).await
            }
        };

        join!(stop_recv, stop_send);
    }
}

struct ChannelBuilder {
    model: String,
    otlp: bool,
}

impl ChannelBuilder {
    #[inline]
    fn new(model: String) -> Self {
        Self { model, otlp: false }
    }

    #[inline]
    fn otlp(self, value: bool) -> Self {
        Self {
            otlp: value,
            ..self
        }
    }

    async fn build(self, channel: &Channel) -> Result<(), ErrorMessage> {
        tracer::init_once_with_default(self.otlp);

        {
            let mut lock = channel.builder.write().await;
            lock.replace(self);
        }

        {
            let mut lock = channel.client.write().await;
            if lock.is_none() {
                lock.replace(try_init_client().await?);
            }
        }

        Ok(())
    }

    async fn build_receiver<'c>(
        &self,
        client: &'c PipeClient<Image>,
        imp: &(impl ?Sized + ChannelSubclass + Plugin),
    ) -> Result<self::recv::Queue, FlowError> {
        let Self { model, otlp: _ } = self;

        let args = QueueArgs {
            client,
            imp,
            label: "subscriber",
            model: model.clone(),
        };

        self::recv::Queue::try_new(args).await
    }

    async fn build_sender<'c>(
        &self,
        client: &'c PipeClient<Image>,
        imp: &(impl ?Sized + ChannelSubclass + Plugin),
    ) -> Result<self::send::Queue, FlowError> {
        let Self { model, otlp: _ } = self;

        let args = QueueArgs {
            client,
            imp,
            label: "publisher",
            model: model.clone(),
        };

        self::send::Queue::try_new(args).await
    }
}

struct QueueArgs<'c, C>
where
    C: ?Sized,
{
    client: &'c PipeClient<Image>,
    imp: &'c C,
    label: &'static str,
    model: String,
}

impl<'c, C> QueueArgs<'c, C>
where
    C: ?Sized + ChannelSubclass + Plugin,
{
    async fn call_client<F, Fut, R>(&self, f: F) -> Result<R, FlowError>
    where
        F: FnOnce(&'c PipeClient<Image>, Name) -> Fut,
        Fut: Future<Output = Result<R>>,
    {
        let model = self.model.parse().map_err(|error| {
            let model = &self.model;
            error!(
                self.imp.cat(),
                imp: self.imp,
                "failed to parse OpenARK model {model:?}: {error}",
            );
            FlowError::Error
        })?;

        f(self.client, model).await.map_err(|error| {
            let label = self.label;
            error!(
                self.imp.cat(),
                imp: self.imp,
                "failed to init OpenARK {label}: {error}",
            );
            FlowError::Eos
        })
    }
}

async fn try_init_client() -> Result<PipeClient<Image>, ErrorMessage> {
    // Do not parse arguments from command line,
    // only use the environment variables.
    let args = PipeClientArgs::try_parse_from::<_, &str>([]).map_err(|error| {
        error_msg!(
            CoreError::Failed,
            ["Failed to parse OpenARK arguments: {error}"]
        )
    })?;

    PipeClient::try_new(&args).await.map_err(|error| {
        error_msg!(
            CoreError::Failed,
            ["Failed to init OpenARK client: {error}"]
        )
    })
}

fn assert_client<'c>(
    client: &'c RwLockReadGuard<'c, Option<PipeClient<Image>>>,
    imp: &(impl ?Sized + ChannelSubclass + Plugin),
) -> Result<&'c PipeClient<Image>, FlowError> {
    client.as_ref().ok_or_else(|| {
        error!(imp.cat(), imp: imp, "OpenARK client is not inited!");
        FlowError::Error
    })
}
