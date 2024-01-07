//! Watches data over specified pubsub topic
use anyhow::{Context, Result};
use vrs::Client;

/// Watch specified topic, optionally following over many values.
pub(crate) async fn run(client: &Client, topic: vrs::KeywordId, follow: bool) -> Result<()> {
    let mut sub = client
        .subscribe(topic)
        .await
        .with_context(|| "Failed to subscribe to {topic}")?;

    loop {
        let form = sub
            .recv()
            .await
            .with_context(|| "Failed to recv on subscription")?;
        println!("{form}");
        if !follow {
            break;
        }
    }

    Ok(())
}
