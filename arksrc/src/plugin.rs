use anyhow::Result;
use gsark_common::client;
use gst::{
    glib::{
        self,
        subclass::types::{ObjectSubclass, ObjectSubclassExt},
        BoolError, Type,
    },
    subclass::prelude::GstObjectImpl,
    Buffer, Caps, CoreError, ErrorMessage, FlowError, LoggableError,
};
use gst_base::{
    prelude::BaseSrcExt,
    subclass::{
        base_src::{BaseSrcImpl, BaseSrcImplExt, CreateSuccess},
        prelude::PushSrcImpl,
    },
};
use gst_video::VideoInfo;
use image::{imageops::FilterType, DynamicImage};
use tokio::{
    runtime::Runtime,
    sync::{mpsc, Mutex, RwLock},
    task::JoinHandle,
};

use crate::{args::Args, state::State};

/// Struct containing all the element data
pub struct Plugin {
    pub(crate) args: RwLock<Args>,
    queue: Mutex<Option<Queue>>,
    runtime: Runtime,
    state: RwLock<State>,
}

impl Default for Plugin {
    fn default() -> Self {
        let runtime = Runtime::new().expect("Tokio runtime should be created");
        let _guard = runtime.enter();

        Self {
            args: RwLock::default(),
            queue: Mutex::default(),
            runtime,
            state: RwLock::default(),
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

    fn fixate(&self, mut caps: Caps) -> Caps {
        // skip if size is already given
        {
            let s = caps.structure(0).unwrap();
            if s.has_field_with_type("width", Type::I32)
                && s.has_field_with_type("height", Type::I32)
            {
                return self.parent_fixate(caps);
            }
        }

        self.runtime.block_on(async {
            // start receiver if not started
            if self.start_receiver().await.is_err() {
                return self.parent_fixate(caps);
            }

            // get sample image
            match self.next().await {
                // update image size
                Some(image) => {
                    caps.truncate();
                    {
                        let caps = caps.make_mut();
                        let s = caps.structure_mut(0).unwrap();
                        s.fixate_field_nearest_int("width", image.width() as i32);
                        s.fixate_field_nearest_int("height", image.height() as i32);
                    }
                    self.parent_fixate(caps)
                }
                None => self.parent_fixate(caps),
            }
        })
    }

    fn set_caps(&self, caps: &Caps) -> Result<(), LoggableError> {
        self.runtime
            .block_on(self.set_caps_async(caps))
            .map_err(Into::into)
    }
}

impl PushSrcImpl for Plugin {
    fn create(
        &self,
        buffer: Option<&mut gst::BufferRef>,
    ) -> Result<gst_base::subclass::base_src::CreateSuccess, gst::FlowError> {
        self.runtime.block_on(async {
            // get video info
            let info = {
                let state = self.state.read().await;
                match &state.info {
                    Some(info) => info.clone(),
                    None => return Err(FlowError::NotNegotiated),
                }
            };
            let width = info.width();
            let height = info.height();

            // load an image frame
            let mut image = match self.next().await {
                Some(image) => image,
                None => return Err(FlowError::Eos),
            };

            // resize image
            if image.width() != width || image.height() != height {
                image = image.resize(width, height, FilterType::Nearest);
            }

            // convert image
            let image = image.to_rgb8();

            // TODO: is buffer used?
            if buffer.is_some() {
                todo!();
            }

            // create a video buffer
            let mut buffer = {
                Buffer::with_size(3 * (image.width() as usize) * (image.height() as usize))
                    .expect("failed to create buffer")
            };

            // fill the buffer with image data
            {
                let buffer = buffer.make_mut();
                buffer.copy_from_slice(0, image.as_raw()).unwrap();
            }

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
    async fn set_caps_async(&self, caps: &Caps) -> Result<(), BoolError> {
        let info = VideoInfo::from_caps(caps)?;

        gst::debug!(
            crate::CAT,
            imp: self,
            "Configuring for caps {caps}",
        );

        self.obj().set_blocksize(info.width() * info.height());

        {
            let mut state = self.state.write().await;
            *state = State { info: Some(info) };
        }
        Ok(())
    }

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

    async fn next(&self) -> Option<DynamicImage> {
        self.queue.lock().await.as_mut()?.next().await
    }
}

struct Queue {
    producer: JoinHandle<Result<()>>,
    rx: mpsc::Receiver<DynamicImage>,
}

impl Queue {
    async fn try_new(runtime: &Runtime, model: String, otlp: bool) -> Result<Self> {
        let client = client::try_init(otlp).await?;
        let mut subscriber = client.subscribe(model.parse()?).await?;

        let (tx, rx) = mpsc::channel(2);
        Ok(Self {
            producer: runtime.spawn(async move {
                // FIXME: add `Drop` flag to always take the latest images
                loop {
                    if let Some(data) = subscriber
                        .read_one()
                        .await?
                        .and_then(|mut msg| msg.payloads.pop())
                        .and_then(|payload| {
                            ::image::load(
                                std::io::Cursor::new(payload.value()?),
                                ::image::ImageFormat::Jpeg,
                            )
                            .ok()
                        })
                    {
                        if tx.send(data).await.is_err() {
                            // Queue is destroying, stop sending.
                            break Ok(());
                        }
                    }
                }
            }),
            rx,
        })
    }

    async fn next(&mut self) -> Option<DynamicImage> {
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
