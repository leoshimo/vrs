use crate::{Error, Extern, Locals, NativeFn, NativeFnOp, Val};

pub(crate) fn help_fn<T: Extern, L: Locals>() -> NativeFn<T, L> {
    NativeFn {
        doc: "(help SYMBOL) - Returns docstring for SYMBOL if any".to_string(),
        func: |_, args| {
            let docstring = match args {
                [Val::Lambda(l)] => l
                    .doc
                    .clone()
                    .unwrap_or("<missing documentation>".to_string()),
                [Val::NativeFn(f)] => f.doc.clone(),
                [Val::NativeAsyncFn(f)] => f.doc.clone(),
                _ => {
                    return Err(Error::UnexpectedArguments(format!(
                        "help expects one callable object as argument - got {:?}",
                        args
                    )))
                } // TODO: Better args display format?
            };

            Ok(NativeFnOp::Return(Val::String(docstring)))
        },
    }
}

// TODO: Test Case for (help SYMBOL)
