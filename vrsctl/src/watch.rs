//! Watches data over specified pubsub topic
use anyhow::{Context, Result};
use vrs::Client;

/// Watch options
pub(crate) struct Opts {
    /// Whether or not watch should keep watching
    pub(crate) follow: bool,
    /// Whether or not watch should clear screen before printing result
    pub(crate) clear: bool,
}

/// Watch specified topic, optionally following over many values.
pub(crate) async fn run(client: &Client, topic: vrs::KeywordId, opts: Opts) -> Result<()> {
    let mut sub = client
        .subscribe(topic)
        .await
        .with_context(|| "Failed to subscribe to {topic}")?;

    loop {
        let form = sub
            .recv()
            .await
            .with_context(|| "Failed to recv on subscription")?;

        if opts.clear {
            clearscreen::clear().with_context(|| "failed to clear screen")?;
        }

        println!("{form}");

        if !opts.follow {
            break;
        }
    }

    Ok(())
}
