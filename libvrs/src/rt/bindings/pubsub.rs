//! Pubsub Bindings
use crate::rt::{
    mailbox::Message,
    proc::ProcessHandle,
    program::{Fiber, NativeAsyncFn, Val},
    pubsub::Subscription,
};
use lyric::{Error, KeywordId, Result};
use tracing::error;

pub(crate) fn subscribe_fn() -> NativeAsyncFn {
    NativeAsyncFn {
        metadata: vec![],
        doc: "(subscribe TOPIC) - Subscribe current process until it exits; receive (:topic_updated TOPIC DATA) messages."
            .to_string(),
        func: |f, args| Box::new(subscribe_impl(f, args)),
    }
}

pub(crate) fn publish_fn() -> NativeAsyncFn {
    NativeAsyncFn {
        metadata: vec![],
        doc: "(publish TOPIC DATA) - Publish DATA over TOPIC, notifying all active subscribers."
            .to_string(),
        func: |f, args| Box::new(publish_impl(f, args)),
    }
}

/// Implementation for (subscribe TOPIC)
async fn subscribe_impl(fiber: &mut Fiber, args: Vec<Val>) -> Result<Val> {
    let topic = match &args[..] {
        [topic] => topic.as_keyword()?.clone(),
        _ => {
            return Err(Error::UnexpectedArguments(
                "subscribe expects one argument".to_string(),
            ))
        }
    };
    let pubsub = fiber
        .locals()
        .pubsub
        .as_ref()
        .ok_or(Error::Runtime("Process has no pubsub handle".to_string()))?;

    let owner = fiber
        .locals()
        .self_handle
        .as_ref()
        .ok_or(Error::Runtime("Process has no self handle".to_string()))?
        .clone();

    let sub = pubsub
        .subscribe(&topic)
        .await
        .map_err(|e| Error::Runtime(format!("Failed to subscribe on pubsub - {e}")))?;

    tokio::spawn(forward_subscription(sub, owner, topic));

    Ok(Val::keyword("ok"))
}

/// Stop even an idle subscription when its owner finishes, fails, or is killed.
/// Waiting on the process exit also cancels a blocked mailbox push.
async fn forward_subscription(mut sub: Subscription, owner: ProcessHandle, topic: KeywordId) {
    let mb = owner.mailbox().clone();
    let forward = async {
        while let Some(ev) = sub.recv().await {
            let msg = Message {
                contents: Val::List(vec![
                    Val::keyword("topic_updated"),
                    Val::Keyword(topic.clone()),
                    ev,
                ]),
            };
            if let Err(e) = mb.push(msg).await {
                error!("Error while pushing subscription event to mailbox - {e}");
                break;
            }
        }
    };
    tokio::select! {
        biased;
        _ = owner.join() => {},
        _ = forward => {},
    }
}

/// Implementation for (publish TOPIC VALUE)
async fn publish_impl(fiber: &mut Fiber, args: Vec<Val>) -> Result<Val> {
    let (topic, val) = match &args[..] {
        [topic, val] => (topic.as_keyword()?.clone(), val.clone()),
        _ => {
            return Err(Error::UnexpectedArguments(
                "publish expects two arguments".to_string(),
            ))
        }
    };
    let pubsub = fiber
        .locals()
        .pubsub
        .as_ref()
        .ok_or(Error::Runtime("Process has no pubsub handle".to_string()))?;

    pubsub
        .publish(&topic, val)
        .await
        .map_err(|e| Error::Runtime(format!("Failed to publish on pubsub - {e}")))?;

    Ok(Val::keyword("ok"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rt::{proc::Process, proc::ProcessSet, pubsub::PubSub};
    use crate::{ProcessId, Program};
    use std::time::Duration;

    #[tokio::test]
    async fn idle_subscription_ends_on_normal_error_and_killed_process_exit() {
        for mode in ["normal", "error", "kill"] {
            let mut processes = ProcessSet::new();
            let source = if mode == "error" {
                "(begin (recv) (error \"expected failure\"))"
            } else {
                "(recv)"
            };
            let owner = Process::from_prog(
                ProcessId::new("test", 1),
                Program::from_expr(source).unwrap(),
            )
            .spawn(&mut processes)
            .unwrap();
            let pubsub = PubSub::spawn();
            let topic = KeywordId::from("idle");
            let subscription = pubsub.subscribe(&topic).await.unwrap();
            let forwarding = tokio::spawn(forward_subscription(subscription, owner.clone(), topic));

            if mode == "kill" {
                owner.kill().await;
            } else {
                owner
                    .notify_message(Message::new(Val::keyword("finish")))
                    .await;
            }
            owner.join().await.unwrap();
            tokio::time::timeout(Duration::from_secs(1), forwarding)
                .await
                .expect("subscription must stop without another publication")
                .unwrap();
        }
    }
}
