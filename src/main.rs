mod client;
mod options;
mod server;

use tracing::{error, Level};
use tracing_subscriber::FmtSubscriber;

fn init_logging() -> anyhow::Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let options: options::Options = argh::from_env();

    // TODO: make this not mutually exclusive
    if options.tracing {
        console_subscriber::init();
    } else {
        init_logging()?;
    }

    let mut handles = Vec::new();

    if options.is_server() {
        handles.push(tokio::spawn(server::run(
            options.server_addr(),
            options.is_client(),
        )));
    }

    if options.is_client() {
        handles.push(tokio::spawn(client::run(options.client_addr())));
    }

    let results = futures::future::join_all(handles).await;
    for result in results {
        match result {
            Ok(result) => {
                if let Err(err) = result {
                    error!("Error: {}", err);
                }
            }
            Err(err) => error!("Error: {}", err),
        }
    }

    // stdin polling will block the client exiting
    // so just force it for now
    if options.is_client() {
        std::process::exit(1);
    }

    Ok(())
}
