#![deny(warnings)]

use anyhow::bail;
use aws_lambda_events::event::sns::{SnsEvent, SnsRecord};
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use tracing::{error, info};

async fn process_record(record: &SnsRecord) -> anyhow::Result<()> {
    // for now just log the message we got
    info!("read message:\n{}\n", record.sns.message);

    Ok(())
}

async fn process_records(records: impl AsRef<[SnsRecord]>) -> anyhow::Result<()> {
    let mut error = false;
    for record in records.as_ref().iter() {
        if let Err(err) = process_record(record).await {
            error!("failed to process record: {}", err);
            error = true;
        }
    }

    if error {
        bail!("One or more records failed");
    }

    Ok(())
}

async fn function_handler(event: LambdaEvent<SnsEvent>) -> Result<(), Error> {
    if let Err(err) = process_records(&event.payload.records).await {
        error!("failed to process records: {}", err);
        return Err(err.into());
    }

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
