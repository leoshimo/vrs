//! Pubsub Bindings
use crate::rt::{
    mailbox::Message,
    program::{Fiber, NativeAsyncFn, Val},
};
use lyric::{Error, Result};
use tracing::error;

pub(crate) fn subscribe_fn() -> NativeAsyncFn {
    NativeAsyncFn {
        func: |f, args| Box::new(subscribe_impl(f, args)),
    }
}

pub(crate) fn publish_fn() -> NativeAsyncFn {
    NativeAsyncFn {
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

    let mb = fiber
        .locals()
        .self_handle
        .as_ref()
        .ok_or(Error::Runtime("Process has no self handle".to_string()))?
        .mailbox()
        .clone();

    let mut sub = pubsub
        .subscribe(&topic)
        .await
        .map_err(|e| Error::Runtime(format!("Failed to subscribe on pubsub - {e}")))?;

    // TODO: Idiom for streaming result from =Subscription= to another sink via async task for proc subs + term subs
    tokio::spawn(async move {
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
            }
        }
    });

    Ok(Val::keyword("ok"))
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
