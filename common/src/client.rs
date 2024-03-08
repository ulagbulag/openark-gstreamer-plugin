use anyhow::Result;
use ark_core::tracer;
use clap::Parser;
use dash_openapi::image::Image;
use dash_pipe_provider::{PipeClient, PipeClientArgs};

pub async fn try_init(otlp: bool) -> Result<PipeClient<Image>> {
    tracer::init_once_with_default(otlp);

    // Do not parse arguments from command line,
    // only use the environment variables.
    let args = PipeClientArgs::try_parse_from::<_, &str>([])?;
    PipeClient::try_new(&args).await
}
