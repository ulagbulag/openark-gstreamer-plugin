mod recv;
mod send;

use std::future::Future;

use anyhow::{anyhow, Result};
use ark_core::tracer;
use async_trait::async_trait;
use bytes::Bytes;
use clap::Parser;
use dash_openapi::image::Image;
use dash_pipe_provider::{Name, PipeClient, PipeClientArgs, PipeMessage, PipePayload};
use gst::{
    debug, error, error_msg,
    glib::{subclass::types::ObjectSubclassExt, ParamSpec, Value},
    info, Buffer, BufferRef, CoreError, ErrorMessage, FlowError, FlowSuccess,
};
use gst_video::gst_base::subclass::base_src::CreateSuccess;
use schemars::JsonSchema;
use tokio::{
    join,
    sync::{MappedMutexGuard, Mutex, RwLock, RwLockReadGuard},
};

use crate::{
    plugin::{base::ArkSubclass, PluginImpl},
    sync,
};

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
        plugin: &(impl ?Sized + PluginImpl),
        id: usize,
        value: &Value,
        pspec: &ParamSpec,
    );
}

pub trait ChannelSubclass
where
    Self: ArkSubclass,
{
    fn channel(&self) -> &Channel;
}

#[async_trait]
pub trait ChannelSubclassExt
where
    Self: ChannelSubclass + PluginImpl,
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

    async fn start_send(&self) -> Result<(), ErrorMessage> {
        match self.channel().init_send(self).await {
            Ok(Some(_)) => Ok(()),
            Ok(None) => Err(error_msg!(
                CoreError::Failed,
                ["OpenARK client is not inited!"]
            )),
            Err(error) => Err(error_msg!(
                CoreError::Failed,
                ["Failed to start OpenARK sender: {error}"]
            )),
        }
    }

    async fn start_recv(&self) -> Result<(), ErrorMessage> {
        match self.channel().init_recv(self).await {
            Ok(Some(_)) => Ok(()),
            Ok(None) => Err(error_msg!(
                CoreError::Failed,
                ["OpenARK client is not inited!"]
            )),
            Err(error) => Err(error_msg!(
                CoreError::Failed,
                ["Failed to start OpenARK receiver: {error}"]
            )),
        }
    }

    #[inline]
    async fn recv(&self) -> Result<Option<Bytes>, FlowError> {
        self.channel().recv(self).await
    }

    async fn recv_buffer(
        &self,
        buffer: Option<&mut BufferRef>,
    ) -> Result<CreateSuccess, FlowError> {
        // load a message
        let message = match self.recv().await? {
            Some(message) => message,
            None => return Err(FlowError::Eos),
        };

        // TODO: is buffer used?
        if buffer.is_some() {
            todo!();
        }

        // create a stream buffer
        let buffer = Buffer::from_slice(message);

        debug!(
            self.cat(),
            imp: self,
            "Produced buffer {buffer:?}",
        );

        Ok(CreateSuccess::NewBuffer(buffer))
    }

    #[inline]
    async fn send(&self, data: PipeMessage<Image>) -> Result<(), FlowError> {
        self.channel().send(self, data).await
    }

    async fn send_buffer(&self, key: String, buffer: &Buffer) -> Result<FlowSuccess, FlowError> {
        // TODO: handle other media types (audio, JSON, plain, ...)
        // TODO: support non-image(video) data using sink Caps and cache it
        // build a payload
        let key_ref = format!("@data:image,{key}");
        let payload = PipePayload::new(
            key,
            Some(Bytes::from(buffer.map_readable().unwrap().to_vec())),
        );

        // build a message
        // TODO: handle other media types (audio, JSON, plain, ...)
        // TODO: to be implemented
        let value = Image::default();
        let message = PipeMessage::with_payloads(vec![payload], value);

        // encode and send
        self.send(message)
            .await
            .map(|()| FlowSuccess::Ok)
            .map_err(|error| {
                error!(
                    self.cat(),
                    imp: self,
                    "{error}",
                );
                FlowError::Error
            })
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
impl<T> ChannelSubclassExt for T where Self: ChannelSubclass + PluginImpl {}

#[derive(Default)]
pub struct Channel {
    builder: RwLock<Option<ChannelBuilder>>,
    client: RwLock<Option<PipeClient<Image>>>,
    recv: Mutex<Option<self::recv::Queue>>,
    send: RwLock<Option<self::send::Queue>>,
}

impl Channel {
    async fn init_recv(
        &self,
        imp: &(impl ?Sized + ChannelSubclassExt + PluginImpl),
    ) -> Result<Option<MappedMutexGuard<'_, self::recv::Queue>>> {
        let mut lock = self.recv.lock().await;
        match lock.as_mut() {
            Some(_) => Ok(Some(sync::mutex::unwrap_lock(lock))),
            None => {
                let builder_lock = self.builder.read().await;
                match builder_lock.as_ref() {
                    Some(builder) => {
                        let client_lock = self.client.read().await;
                        let client = assert_client(&client_lock)?;

                        lock.replace(builder.build_receiver(client, imp).await?);
                        drop(client_lock);
                        drop(builder_lock);

                        Ok(Some(sync::mutex::unwrap_lock(lock)))
                    }
                    None => Ok(None),
                }
            }
        }
    }

    async fn init_send(
        &self,
        imp: &(impl ?Sized + ChannelSubclassExt + PluginImpl),
    ) -> Result<Option<RwLockReadGuard<'_, self::send::Queue>>> {
        let lock = self.send.read().await;
        match lock.as_ref() {
            Some(_) => Ok(Some(sync::rwlock::unwrap_lock(lock))),
            None => {
                drop(lock);

                let builder_lock = self.builder.read().await;
                match builder_lock.as_ref() {
                    Some(builder) => {
                        let client_lock = self.client.read().await;
                        let client = assert_client(&client_lock)?;

                        let mut lock = self.send.write().await;
                        lock.replace(builder.build_sender(client, imp).await?);
                        drop(client_lock);
                        drop(builder_lock);
                        drop(lock);

                        let lock = self.send.read().await;
                        Ok(Some(sync::rwlock::unwrap_lock(lock)))
                    }
                    None => Ok(None),
                }
            }
        }
    }

    async fn recv(
        &self,
        imp: &(impl ?Sized + ChannelSubclassExt + PluginImpl),
    ) -> Result<Option<Bytes>, FlowError> {
        let maybe_queue = self.init_recv(imp).await.map_err(|error| {
            error!(imp.cat(), imp: imp, "{error}");
            FlowError::Error
        })?;

        match maybe_queue {
            Some(mut queue) => Ok(queue.recv().await),
            None => Ok(None),
        }
    }

    async fn send(
        &self,
        imp: &(impl ?Sized + ChannelSubclassExt + PluginImpl),
        data: PipeMessage<Image>,
    ) -> Result<(), FlowError> {
        let maybe_queue = self.init_send(imp).await.map_err(|error| {
            error!(imp.cat(), imp: imp, "{error}");
            FlowError::Error
        })?;

        match maybe_queue {
            Some(queue) => queue.send(imp, data).await,
            None => Err(FlowError::Eos),
        }
    }

    async fn stop(&self, imp: &(impl ?Sized + ChannelSubclassExt + PluginImpl)) {
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
        imp: &(impl ?Sized + ChannelSubclassExt + PluginImpl),
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
        imp: &(impl ?Sized + ChannelSubclassExt + PluginImpl),
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
    C: ?Sized + ChannelSubclassExt + PluginImpl,
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

pub async fn try_init_client<T>() -> Result<PipeClient<T>, ErrorMessage>
where
    T: JsonSchema,
{
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
) -> Result<&'c PipeClient<Image>> {
    client
        .as_ref()
        .ok_or_else(|| anyhow!("OpenARK client is not inited!"))
}
