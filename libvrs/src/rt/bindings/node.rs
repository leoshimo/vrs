//! Node configuration bindings.

use crate::{Extern, Fiber, NativeAsyncFn, Val};
use lyric::{kwargs, Error, KeywordId, Result};

pub(crate) fn eval_remote_fn() -> NativeAsyncFn {
    // VRS host binding, not a Lyric special form: the VM's normal native-async
    // path awaits this future and resumes the calling fiber with its result.
    // Registry synchronization happens in remote.rs before that future resolves.
    NativeAsyncFn {
        metadata: vec![],
        doc: "(eval_remote NODE FORM) - Evaluate code in a fresh process on NODE. Wait for its result and local visibility of the target's completed registrations.".into(),
        func: |fiber, args| Box::new(async move {
            let [Val::String(node), code] = args.as_slice() else {
                return Err(Error::UnexpectedArguments("eval_remote expects a node string and a code value".into()));
            };
            // Validate the same transferable-code contract for local targets.
            let code = crate::rt::peer::WireVal::from_val(code.clone()).map_err(|e| Error::Runtime(e.to_string()))?.into_val().map_err(|e| Error::Runtime(e.to_string()))?;
            let result = if node == &fiber.locals().node_name {
                crate::rt::remote::evaluate(fiber.locals().kernel.clone().ok_or_else(|| Error::Runtime("no kernel".into()))?, code).await
                    .and_then(crate::rt::peer::WireVal::from_val).and_then(crate::rt::peer::WireVal::into_val)
            } else {
                fiber.locals().peers.as_ref().ok_or_else(|| Error::Runtime("no node transport".into()))?.eval(node.clone(), code).await
            };
            result.map_err(|e| match e { crate::Error::EvaluationError(e) => e, e => Error::Runtime(e.to_string()) })
        }),
    }
}

pub(crate) fn wait_srv_fn() -> NativeAsyncFn {
    NativeAsyncFn {
        metadata: vec![],
        doc: "(wait_srv NAME [:pid PID] [:timeout SECONDS]) - Wait until the local registry resolves NAME, optionally to an exact PID. Without a timeout, wait until matched or cancelled.".into(),
        func: |fiber, args| Box::new(async move {
            let Some(Val::Keyword(name)) = args.first() else {
                return Err(Error::UnexpectedArguments("wait_srv expects a service keyword".into()));
            };
            let mut pid = None;
            let mut timeout = None;
            let mut options = args[1..].chunks_exact(2);
            for pair in &mut options {
                match pair {
                    [Val::Keyword(key), Val::Extern(Extern::ProcessId(value))] if key.as_str() == "pid" && pid.is_none() => pid = Some(value.clone()),
                    [Val::Keyword(key), Val::Int(value)] if key.as_str() == "timeout" && *value >= 0 && timeout.is_none() => timeout = Some(std::time::Duration::from_secs(*value as u64)),
                    _ => return Err(Error::UnexpectedArguments("wait_srv options are :pid PID and :timeout NONNEGATIVE-SECONDS".into())),
                }
            }
            if !options.remainder().is_empty() { return Err(Error::UnexpectedArguments("wait_srv option is missing its value".into())); }
            let registry = fiber.locals().registry.as_ref().ok_or_else(|| Error::Runtime("no registry".into()))?;
            if timeout == Some(std::time::Duration::ZERO) {
                let entry = registry.lookup(name.clone()).await.map_err(|e| Error::Runtime(e.to_string()))?;
                return match entry {
                    Some(entry) if pid.as_ref().is_none_or(|pid| *pid == entry.pid()) => Ok(Val::Extern(Extern::ProcessId(entry.pid()))),
                    _ => Err(Error::Runtime(format!("timed out waiting for {name}"))),
                };
            }
            let wait = registry.wait_for(name.clone(), pid);
            let pid = match timeout {
                Some(duration) => tokio::time::timeout(duration, wait).await.map_err(|_| Error::Runtime(format!("timed out waiting for {name}")))?,
                None => wait.await,
            }.map_err(|e| Error::Runtime(e.to_string()))?;
            Ok(Val::Extern(Extern::ProcessId(pid)))
        }),
    }
}

pub(crate) fn configure_fn() -> NativeAsyncFn {
    NativeAsyncFn {
        metadata: vec![],
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
