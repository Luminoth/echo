use aws_lambda_events::event::sns::SnsEvent;
use lambda_runtime::{run, service_fn, Error, LambdaEvent};

async fn function_handler(_event: LambdaEvent<SnsEvent>) -> Result<(), Error> {
    // Extract some useful information from the request

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .without_time()
        .init();

    run(service_fn(function_handler)).await
}
