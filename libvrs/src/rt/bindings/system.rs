//! Host System Bindings

use crate::rt::program::{NativeAsyncFn, NativeFn, NativeFnOp, Val};
use lyric::{Error, Result};
use serde_json::Value as JsonValue;
use tokio::process::Command;
use tracing::debug;

/// Binding for exec
pub(crate) fn exec_fn() -> NativeAsyncFn {
    NativeAsyncFn {
        doc: "(exec PROG ARG1 ARG2 ... ARGN) - Execute PROG and return (:exit CODE :stdout STRING :stderr STRING).".to_string(),
        func: |_, args| Box::new(exec_impl(args)),
    }
}

/// Binding for decode
pub(crate) fn decode_fn() -> NativeFn {
    NativeFn {
        doc: "(decode FORMAT STRING [:columns COLUMNS]) - Decode external text as :json, :tsv, or :lines. :tsv optionally maps fields to keyword COLUMNS.".to_string(),
        func: |_, args| decode_impl(args),
    }
}

/// Binding for shell_expand
pub(crate) fn shell_expand_fn() -> NativeFn {
    NativeFn {
        doc: "(shell_expand STRING) - Expand STRING using standard shell filename expansion."
            .to_string(),
        func: |_, args| {
            let path = match args {
                [Val::String(s)] => s,
                _ => {
                    return Err(Error::UnexpectedArguments(
                        "shell_expand expects one argument".to_string(),
                    ))
                }
            };
            let path = shellexpand::tilde(path).to_string();
            Ok(NativeFnOp::Return(Val::String(path)))
        },
    }
}

/// Implementation of (exec PROG ARGS...)
async fn exec_impl(args: Vec<Val>) -> Result<Val> {
    let (prog, args) = args.split_first().ok_or(Error::UnexpectedArguments(
        " Unexpected arguments to exec = (exec PROG [ARGS...])".to_string(),
    ))?;

    let prog = match prog {
        Val::String(s) => s.clone(),
        _ => {
            return Err(Error::UnexpectedArguments(
                "Expected string as first argument".to_string(),
            ))
        }
    };

    let args = args
        .iter()
        .map(|a| match a {
            Val::String(s) => Ok(s.clone()),
            _ => Err(Error::UnexpectedArguments(
                "exec can handle string arguments only".to_string(),
            )),
        })
        .collect::<Result<Vec<_>>>()?;

    debug!("exec {:?} {:?}", &prog, &args);

    let output = Command::new(prog.clone())
        .args(args.clone())
        .output()
        .await
        .map_err(|e| Error::Runtime(format!("{e}")))?;

    let exit = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8(output.stdout)
        .map_err(|e| Error::Runtime(format!("stdout was not valid UTF-8: {e}")))?;
    let stderr = String::from_utf8(output.stderr)
        .map_err(|e| Error::Runtime(format!("stderr was not valid UTF-8: {e}")))?;

    debug!("exec {:?} {:?} - {:?}", prog, args, output.status);
    Ok(Val::List(vec![
        Val::keyword("exit"),
        Val::Int(exit),
        Val::keyword("stdout"),
        Val::String(stdout),
        Val::keyword("stderr"),
        Val::String(stderr),
    ]))
}

/// Implementation of (decode FORMAT STRING [:columns COLUMNS])
fn decode_impl(args: &[Val]) -> Result<NativeFnOp> {
    let (format, input, options) = match args {
        [Val::Keyword(format), Val::String(input), options @ ..] => {
            (format.as_str(), input.as_str(), options)
        }
        _ => {
            return Err(Error::UnexpectedArguments(
                "decode expects a format keyword and string".to_string(),
            ))
        }
    };

    let value = match format {
        "json" if options.is_empty() => decode_json(input)?,
        "lines" if options.is_empty() => decode_lines(input),
        "tsv" => decode_tsv(input, decode_columns(options)?)?,
        "json" | "lines" => {
            return Err(Error::UnexpectedArguments(format!(
                ":{format} does not accept options"
            )))
        }
        _ => {
            return Err(Error::UnexpectedArguments(format!(
                "unsupported decode format :{format}"
            )))
        }
    };

    Ok(NativeFnOp::Return(value))
}

fn decode_json(input: &str) -> Result<Val> {
    let value = serde_json::from_str(input)
        .map_err(|e| Error::Runtime(format!("failed to decode JSON: {e}")))?;
    json_to_val(value)
}

fn json_to_val(value: JsonValue) -> Result<Val> {
    match value {
        JsonValue::Null => Ok(Val::Nil),
        JsonValue::Bool(value) => Ok(Val::Bool(value)),
        JsonValue::String(value) => Ok(Val::String(value)),
        JsonValue::Number(value) => {
            if let Some(value) = value.as_i64().and_then(|value| i32::try_from(value).ok()) {
                Ok(Val::Int(value))
            } else if let Some(value) = value.as_u64().and_then(|value| i32::try_from(value).ok()) {
                Ok(Val::Int(value))
            } else if let Some(value) = value.as_f64().filter(|value| value.fract() == 0.0) {
                i32::try_from(value as i64)
                    .map(Val::Int)
                    .or_else(|_| Ok(Val::String(value.to_string())))
            } else {
                // Lyric currently has integers but no floating-point or arbitrary-precision
                // number type. Preserve unsupported JSON numbers as strings rather than
                // rejecting an otherwise useful document.
                Ok(Val::String(value.to_string()))
            }
        }
        JsonValue::Array(values) => values
            .into_iter()
            .map(json_to_val)
            .collect::<Result<Vec<_>>>()
            .map(Val::List),
        JsonValue::Object(values) => {
            let mut result = Vec::with_capacity(values.len() * 2);
            for (key, value) in values {
                result.push(Val::keyword(&key));
                result.push(json_to_val(value)?);
            }
            Ok(Val::List(result))
        }
    }
}

fn decode_lines(input: &str) -> Val {
    Val::List(
        input
            .lines()
            .filter(|line| !line.is_empty())
            .map(Val::string)
            .collect(),
    )
}

fn decode_columns(options: &[Val]) -> Result<Option<Vec<String>>> {
    match options {
        [] => Ok(None),
        [Val::Keyword(option), Val::List(columns)] if option.as_str() == "columns" => columns
            .iter()
            .map(|column| match column {
                Val::Keyword(column) => Ok(column.as_str().to_string()),
                _ => Err(Error::UnexpectedArguments(
                    ":columns must contain keywords".to_string(),
                )),
            })
            .collect::<Result<Vec<_>>>()
            .map(Some),
        _ => Err(Error::UnexpectedArguments(
            ":tsv accepts only :columns '(:column ...)".to_string(),
        )),
    }
}

fn decode_tsv(input: &str, columns: Option<Vec<String>>) -> Result<Val> {
    let rows = input
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let fields = match &columns {
                Some(columns) if !columns.is_empty() => {
                    let mut fields = line
                        .splitn(columns.len(), '\t')
                        .map(str::to_string)
                        .collect::<Vec<_>>();
                    fields.resize(columns.len(), String::new());
                    fields
                }
                _ => line.split('\t').map(str::to_string).collect(),
            };

            match &columns {
                Some(columns) => Ok(Val::List(
                    columns
                        .iter()
                        .zip(fields)
                        .flat_map(|(column, field)| [Val::keyword(column), Val::String(field)])
                        .collect(),
                )),
                None => Ok(Val::List(fields.into_iter().map(Val::String).collect())),
            }
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(Val::List(rows))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn exec_captures_exact_output_and_nonzero_exit() {
        let value = exec_impl(vec![
            Val::string("sh"),
            Val::string("-c"),
            Val::string("printf ' out\\n'; printf ' err\\n' >&2; exit 7"),
        ])
        .await
        .unwrap();

        assert_eq!(
            value,
            Val::List(vec![
                Val::keyword("exit"),
                Val::Int(7),
                Val::keyword("stdout"),
                Val::string(" out\n"),
                Val::keyword("stderr"),
                Val::string(" err\n"),
            ])
        );
    }

    #[test]
    fn decodes_json_to_lyric_values() {
        let value =
            decode_json(r#"[{"id":42,"title":"hello","visible":true,"other":null,"opacity":0.5}]"#)
                .unwrap();

        let Val::List(items) = value else {
            panic!("expected a list");
        };
        let [Val::List(item)] = items.as_slice() else {
            panic!("expected one object");
        };

        let get = |key: &str| {
            item.windows(2)
                .find(|pair| pair[0] == Val::keyword(key))
                .map(|pair| pair[1].clone())
        };
        assert_eq!(get("id"), Some(Val::Int(42)));
        assert_eq!(get("title"), Some(Val::string("hello")));
        assert_eq!(get("visible"), Some(Val::Bool(true)));
        assert_eq!(get("other"), Some(Val::Nil));
        assert_eq!(get("opacity"), Some(Val::string("0.5")));
    }

    #[test]
    fn decodes_lines_without_a_trailing_empty_record() {
        assert_eq!(
            decode_lines("one\r\ntwo\n\n"),
            Val::List(vec![Val::string("one"), Val::string("two")])
        );
    }

    #[test]
    fn decodes_tsv_with_declarative_columns() {
        assert_eq!(
            decode_tsv(
                "1\tFirst title\n2\tSecond\ttitle\n",
                Some(vec!["id".into(), "title".into()])
            )
            .unwrap(),
            Val::List(vec![
                Val::List(vec![
                    Val::keyword("id"),
                    Val::string("1"),
                    Val::keyword("title"),
                    Val::string("First title"),
                ]),
                Val::List(vec![
                    Val::keyword("id"),
                    Val::string("2"),
                    Val::keyword("title"),
                    Val::string("Second\ttitle"),
                ]),
            ])
        );
    }
}
