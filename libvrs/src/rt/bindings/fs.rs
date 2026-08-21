//! File System bindings for VRS Processes

use crate::{Fiber, NativeAsyncFn, ProcessResult, Program, Val};
use lyric::{parse, Error, Form, Result};
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncReadExt, AsyncWriteExt},
};

pub(crate) fn fread_fn() -> NativeAsyncFn {
    NativeAsyncFn {
        doc: "(fread PATH) - Read the symbolic expression from file at PATH".to_string(),
        func: |f, args| Box::new(fread_impl(f, args)),
    }
}

async fn fread_impl(_fiber: &mut Fiber, args: Vec<Val>) -> Result<Val> {
    let path = match &args[..] {
        [Val::String(s)] => shellexpand::tilde(s).to_string(),
        e => {
            return Err(Error::UnexpectedArguments(format!(
                "fread expects first argument to be string. Got {:?}",
                e
            )))
        }
    };

    let mut file = File::open(path)
        .await
        .map_err(|e| Error::Runtime(format!("Failed to open file - {e}")))?;

    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .await
        .map_err(|e| Error::Runtime(format!("Failed to read file - {e}")))?;

    let val = Val::from(parse(&contents)?);

    Ok(val)
}

pub(crate) fn fdump_fn() -> NativeAsyncFn {
    NativeAsyncFn {
        doc: "(fdump PATH FORM) - Dump the symbolic expression FORM to file at PATH".to_string(),
        func: |f, args| Box::new(fdump_impl(f, args)),
    }
}

pub(crate) fn run_script_fn() -> NativeAsyncFn {
    NativeAsyncFn {
        doc: "(run PATH) - Evaluate every top-level form in PATH in a fresh process and wait for it to finish"
            .to_string(),
        func: |f, args| Box::new(run_script_impl(f, args)),
    }
}

async fn run_script_impl(fiber: &mut Fiber, args: Vec<Val>) -> Result<Val> {
    let path = match &args[..] {
        [Val::String(s)] => shellexpand::tilde(s).to_string(),
        _ => {
            return Err(Error::UnexpectedArguments(
                "run expects one string path".to_string(),
            ))
        }
    };

    let contents = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| Error::Runtime(format!("Failed to read script {path} - {e}")))?;
    let program = Program::from_script(&contents)?;
    let kernel = fiber
        .locals()
        .kernel
        .as_ref()
        .and_then(|kernel| kernel.upgrade())
        .ok_or_else(|| Error::Runtime("Kernel is missing for process".to_string()))?;
    let exit = kernel
        .spawn_prog(program)
        .await
        .map_err(|e| Error::Runtime(format!("Failed to run script {path} - {e}")))?
        .join()
        .await
        .map_err(|e| Error::Runtime(format!("Failed waiting for script {path} - {e}")))?;

    match exit.status {
        Ok(ProcessResult::Done(value)) => Ok(value),
        Ok(ProcessResult::Cancelled) => Err(Error::Runtime(format!("Script {path} was cancelled"))),
        Err(e) => Err(Error::Runtime(format!("Script {path} failed - {e}"))),
    }
}

async fn fdump_impl(_fiber: &mut Fiber, args: Vec<Val>) -> Result<Val> {
    let (path, val) = match &args[..] {
        [Val::String(s), val] => (shellexpand::tilde(s).to_string(), val),
        a => {
            return Err(Error::UnexpectedArguments(format!(
                "fread expects first argument to be string. Got {:?}",
                a
            )))
        }
    };

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .await
        .map_err(|e| Error::Runtime(format!("Failed to open file - {e}")))?;

    let val_str = Form::try_from(val.clone())?.to_string();
    file.write_all(val_str.as_bytes())
        .await
        .map_err(|e| Error::Runtime(format!("Failed to write to file - {e}")))?;

    file.flush()
        .await
        .map_err(|e| Error::Runtime(format!("Failed to flush write to file - {e}")))?;

    Ok(Val::keyword("ok"))
}

// TODO: Test Cases for fread
// TODO: Test Cases for fdump
