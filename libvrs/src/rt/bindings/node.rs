//! Node configuration bindings.

use crate::{Fiber, NativeAsyncFn, Val};
use lyric::{kwargs, Error, KeywordId, Result};

pub(crate) fn configure_fn() -> NativeAsyncFn {
    NativeAsyncFn {
        doc: "(configure :nodes '(ENDPOINT ...)) - Nonblockingly add tcp:// or ssh:// node links"
            .to_string(),
        func: |fiber, args| Box::new(configure_impl(fiber, args)),
    }
}

async fn configure_impl(fiber: &mut Fiber, args: Vec<Val>) -> Result<Val> {
    let nodes = kwargs::get(&args, &KeywordId::from("nodes")).ok_or_else(|| {
        Error::UnexpectedArguments("configure expects :nodes followed by a list".to_string())
    })?;
    let nodes = match nodes {
        Val::List(nodes) => nodes
            .iter()
            .map(|node| match node {
                Val::String(node) => Ok(node.clone()),
                _ => Err(Error::UnexpectedArguments(
                    ":nodes entries must be strings".to_string(),
                )),
            })
            .collect::<Result<Vec<_>>>()?,
        _ => {
            return Err(Error::UnexpectedArguments(
                ":nodes must be a list of strings".to_string(),
            ))
        }
    };

    let peers = fiber
        .locals()
        .peers
        .as_ref()
        .ok_or_else(|| Error::Runtime("This process has no node transport".to_string()))?;
    peers
        .configure(nodes)
        .await
        .map_err(|e| Error::Runtime(format!("{e}")))?;
    Ok(Val::keyword("ok"))
}
