use crate::{Error, Extern, Locals, NativeFn, NativeFnOp, Result, Val};
use dyn_fmt::AsStrFormatExt;

pub(crate) fn str_fn<T: Extern, L: Locals>() -> NativeFn<T, L> {
    NativeFn {
        doc: "(str ARG1 ARG2 ... ARGN) - Returns a new string by concatenating each argument coerced into string.\
              Arguments are optional.".to_string(),
        func: |_, args| {
            let mut result = String::new();
            for v in args {
                result += &v.as_string_coerce()?;
            }
            Ok(NativeFnOp::Return(Val::String(result)))
        },
    }
}

pub(crate) fn display_fn<T: Extern, L: Locals>() -> NativeFn<T, L> {
    NativeFn {
        doc: "(display ARG1 ARG2 ... ARGN) - Returns a new string by concatenating each argument as a display string.".to_string(),
        func: |_, args| {
            let mut result = String::new();
            for v in args {
                result += &v.to_string();
            }
            Ok(NativeFnOp::Return(Val::String(result)))
        },
    }
}

pub(crate) fn join_fn<T: Extern, L: Locals>() -> NativeFn<T, L> {
    NativeFn {
        doc: "(join SEP ARG1 ARG2 ... ARGN) - Returns a new string by concatenating each argument separated by SEP.".to_string(),
        func: |_, args| {
            let separator = args
                .first()
                .ok_or(Error::UnexpectedArguments(
                    "First argument should be string separator".to_string(),
                ))?
                .as_string()?;

            let str_args = args
                .iter()
                .skip(1)
                .map(|v| v.as_string_coerce())
                .collect::<Result<Vec<_>>>()?;

            let result = str_args.join(separator);
            Ok(NativeFnOp::Return(Val::String(result)))
        },
    }
}

pub(crate) fn split_fn<T: Extern, L: Locals>() -> NativeFn<T, L> {
    NativeFn {
        doc: "(split SEP STR) - Returns a list separating string STR by SEP.".to_string(),
        func: |_, args| {
            let substrings = match args {
                [Val::String(sep), Val::String(string)] => string.split(sep),
                _ => {
                    return Err(Error::UnexpectedArguments(
                        "(split SEP STR) expects SEP and STR as arguments".to_string(),
                    ))
                }
            };
            let result = substrings.map(|s| Val::string(s)).collect::<Vec<_>>();
            Ok(NativeFnOp::Return(Val::List(result)))
        },
    }
}

pub(crate) fn format_fn<T: Extern, L: Locals>() -> NativeFn<T, L> {
    NativeFn {
        doc: "(format FORMAT ARG1 ARG2 ... ARGN) - Returns a new string by templating FORMAT with arguments coerced into strings.".to_string(),
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
        doc: "(read STRING) - Returns a symbolic expression by parsing STRING.".to_string(),
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
