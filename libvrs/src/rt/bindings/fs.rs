//! File System bindings for VRS Processes

use crate::{Fiber, NativeAsyncFn, Val};
use lyric::{parse, Error, Result};
use tokio::{fs::File, io::AsyncReadExt};

pub(crate) fn fread_fn() -> NativeAsyncFn {
    NativeAsyncFn {
        doc: "(fread FILE_NAME) - Read the symbolic expression in FILE_NAME".to_string(),
        func: |f, args| Box::new(fread_impl(f, args)),
    }
}

async fn fread_impl(_fiber: &mut Fiber, args: Vec<Val>) -> Result<Val> {
    let file_name = match &args[..] {
        [Val::String(s)] => shellexpand::tilde(s).to_string(),
        a => {
            return Err(Error::UnexpectedArguments(format!(
                "fread expects first argument to be string. Got {:?}",
                a
            )))
        }
    };

    let mut file = File::open(file_name)
        .await
        .map_err(|e| Error::Runtime(format!("Failed to open file - {e}")))?;

    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .await
        .map_err(|e| Error::Runtime(format!("Failed to read file - {e}")))?;

    let val = Val::from(parse(&contents)?);

    Ok(val)
}

// TODO: Test Cases for fread
