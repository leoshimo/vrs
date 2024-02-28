//! File System bindings for VRS Processes

use crate::{Fiber, NativeAsyncFn, Val};
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
