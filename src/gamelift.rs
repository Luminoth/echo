use aws_sdk_gamelift::{config, Client, Endpoint};
use http::Uri;

pub async fn new_client(local: bool) -> Client {
    let shared_config = aws_config::from_env().load().await;

    let mut config = config::Builder::from(&shared_config);
    if local {
        config = config.endpoint_resolver(Endpoint::immutable(Uri::from_static(
            "http://localhost:8080",
        )));
    }
    let config = config.build();

    Client::from_conf(config)
}
