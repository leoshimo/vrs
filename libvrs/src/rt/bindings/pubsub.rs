//! Pubsub Bindings
use crate::rt::proc_io::IOCmd;
use crate::{Extern, NativeFn, NativeFnOp, Val};
use lyric::Error;

pub(crate) fn subscribe_fn() -> NativeFn {
    NativeFn {
        func: |_, args| {
            let topic = match args {
                [topic] => topic.as_keyword()?.clone(),
                _ => {
                    return Err(Error::UnexpectedArguments(
                        "subscribe expects one argument".to_string(),
                    ))
                }
            };
            Ok(NativeFnOp::Yield(Val::Extern(Extern::IOCmd(Box::new(
                IOCmd::Subscribe(topic),
            )))))
        },
    }
}

pub(crate) fn unsubscribe_fn() -> NativeFn {
    NativeFn {
        func: |_, _args| Ok(NativeFnOp::Return(Val::Nil)),
    }
}

pub(crate) fn publish_fn() -> NativeFn {
    NativeFn {
        func: |_, args| {
            let (topic, val) = match args {
                [topic, val] => (topic.as_keyword()?.clone(), val.clone()),
                _ => {
                    return Err(Error::UnexpectedArguments(
                        "publish expects two arguments".to_string(),
                    ))
                }
            };
            Ok(NativeFnOp::Yield(Val::Extern(Extern::IOCmd(Box::new(
                IOCmd::Publish(topic, val),
            )))))
        },
    }
}
