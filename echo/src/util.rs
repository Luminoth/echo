use tokio::sync::watch;

pub async fn wait_for_signal(mut receiver: watch::Receiver<bool>) -> anyhow::Result<()> {
    loop {
        receiver.changed().await?;
        if *receiver.borrow() {
            break;
        }

        tokio::task::yield_now().await;
    }

    Ok(())
}
