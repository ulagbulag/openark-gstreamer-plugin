use anyhow::Result;
use clap::Parser;
use ark_core::tracer;
use dash_pipe_provider::{PipeClient, PipeClientArgs};

pub async fn try_init(otlp: bool) -> Result<PipeClient> {
    tracer::init_once_with_default(otlp);

    // Do not parse arguments from command line,
    // only use the environment variables.
    let args = PipeClientArgs::try_parse_from::<_, &str>([])?;
    PipeClient::try_new(&args).await
}
