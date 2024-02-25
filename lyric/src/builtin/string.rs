use crate::{Error, Extern, Locals, NativeFn, NativeFnOp, Result, Val};
use dyn_fmt::AsStrFormatExt;

pub(crate) fn str_fn<T: Extern, L: Locals>() -> NativeFn<T, L> {
    NativeFn {
        func: |_, args| {
            let mut result = String::new();
            for v in args {
                result += &v.as_string_coerce()?;
            }
            Ok(NativeFnOp::Return(Val::String(result)))
        },
    }
}

pub(crate) fn format_fn<T: Extern, L: Locals>() -> NativeFn<T, L> {
    NativeFn {
        func: |_, args| {
            let format = args
                .first()
                .ok_or(Error::UnexpectedArguments(
                    "First argument should be format string".to_string(),
                ))?
                .as_string()?;

            let str_args = args
                .iter()
                .skip(1)
                .map(|v| v.as_string_coerce())
                .collect::<Result<Vec<_>>>()?;

            let result = format.format(str_args.as_slice());
            Ok(NativeFnOp::Return(Val::String(result)))
        },
    }
}

pub(crate) fn read_fn<T: Extern, L: Locals>() -> NativeFn<T, L> {
    NativeFn {
        func: |_, args| {
            let expr = match args {
                [Val::String(s)] => s,
                _ => {
                    return Err(Error::UnexpectedArguments(
                        "read accepts a single string argument".to_string(),
                    ))
                }
            };

            let val: Val<T, L> = crate::parse(expr)
                .map_err(|e| Error::Runtime(format!("{e}")))?
                .into();
            Ok(NativeFnOp::Return(val))
        },
    }
}

// TODO: Test cases for str:
// (str "a " "b " "c")
// (str "a" " " "b" " " "c") # => "a b c"
// (str 5) # => 5

// TODO: Test cases for `format`

// TODO: Test cases for `read`
