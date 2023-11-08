//! Service Bindings
//! See also [super::registry]
use lyric::{compile, kwargs, Error, KeywordId, Result};

use crate::rt::proc_io::IOCmd;
use crate::rt::program::{Extern, Fiber, NativeFn, NativeFnOp, Val};

/// Binding for register
pub(crate) fn register_fn() -> NativeFn {
    NativeFn {
        func: |_, args| {
            let keyword = match args {
                [Val::Keyword(k)] => k.clone(),
                _ => {
                    return Err(Error::UnexpectedArguments(
                        "register expects single keyword argument".to_string(),
                    ))
                }
            };

            Ok(NativeFnOp::Yield(Val::Extern(Extern::IOCmd(Box::new(
                IOCmd::RegisterAsService(keyword),
            )))))
        },
    }
}

/// Binding for ls-srv
pub(crate) fn ls_srv_fn() -> NativeFn {
    NativeFn {
        func: |_, args| {
            if !args.is_empty() {
                return Err(Error::UnexpectedArguments(
                    "ls-srv expects single keyword argument".to_string(),
                ));
            }
            Ok(NativeFnOp::Yield(Val::Extern(Extern::IOCmd(Box::new(
                IOCmd::ListServices,
            )))))
        },
    }
}

/// Binding for find-srv
pub(crate) fn find_srv_fn() -> NativeFn {
    NativeFn {
        func: |_, args| {
            let keyword = match args {
                [Val::Keyword(k)] => k.clone(),
                _ => {
                    return Err(Error::UnexpectedArguments(
                        "find-srv expects single keyword argument".to_string(),
                    ))
                }
            };

            Ok(NativeFnOp::Yield(Val::Extern(Extern::IOCmd(Box::new(
                IOCmd::FindService(keyword),
            )))))
        },
    }
}

/// Binding for `srv`
pub(crate) fn srv_fn() -> NativeFn {
    NativeFn { func: srv }
}

// TODO: Define as lisp macro
fn srv(f: &mut Fiber, args: &[Val]) -> Result<NativeFnOp> {
    // Expand
    //     (srv :name :SRV_NAME :exports '(sym_a sym_b))
    // to
    //     (begin
    //         (register :launcher)
    //         (loop
    //             (def (r src msg) (recv))
    //             (def resp
    //                 (try (match msg
    //                     ((:sym_a arg1 arg2) (sym_a arg1 arg2))
    //                     ((:sym_b) (sym_b))
    //                     (_ '(:err "Unrecognized message")))))
    //             (send src (list r resp))))

    let name = kwargs::get(args, &KeywordId::from("name")).ok_or(Error::UnexpectedArguments(
        "Missing :name keyword argument".to_string(),
    ))?;

    let exports = kwargs::get(args, &KeywordId::from("exports")).ok_or(
        Error::UnexpectedArguments("Missing :exports keyword argument".to_string()),
    )?;
    let exports = match exports {
        Val::List(symbols) => Ok(symbols),
        _ => Err(Error::UnexpectedArguments(
            ":exports keyword argument must be a list".to_string(),
        )),
    }?;
    let exports = exports
        .into_iter()
        .map(|e| match e {
            Val::Symbol(s) => Ok(s),
            _ => Err(Error::UnexpectedArguments(
                "Forms in :exports list should be symbols".to_string(),
            )),
        })
        .collect::<Result<Vec<_>>>()?;

    let register_form = Val::List(vec![Val::symbol("register"), name]);

    let mut match_form = vec![Val::symbol("match"), Val::symbol("msg")];
    for sym in exports {
        let val = f
            .cur_env()
            .lock()
            .unwrap()
            .get(&sym)
            .ok_or(Error::InvalidExpression(format!(
                "No symbol bound to {}",
                sym
            )))?;
        let lambda = match val {
            Val::Lambda(l) => Ok(l),
            _ => Err(Error::UnexpectedArguments(format!(
                "{} is not a lambda - found {}",
                sym, val
            ))),
        }?;
        let pattern = std::iter::once(Val::Keyword(sym.clone().to_keyword()))
            .chain(lambda.params.iter().map(|v| Val::Symbol(v.clone())))
            .collect::<Vec<_>>();
        let body = std::iter::once(Val::Symbol(sym))
            .chain(lambda.params.into_iter().map(Val::Symbol))
            .collect::<Vec<_>>();
        match_form.push(Val::List(vec![Val::List(pattern), Val::List(body)]));
    }

    // catch-all
    match_form.push(Val::List(vec![
        Val::symbol("_"),
        Val::List(vec![
            Val::symbol("quote"),
            Val::List(vec![
                Val::keyword("err"),
                Val::string("Unrecognized message"),
            ]),
        ]),
    ]));

    // TODO: Rust macros plz
    let ast = Val::List(vec![
        Val::symbol("begin"),
        register_form,
        Val::List(vec![
            Val::symbol("loop"),
            // (def (r src msg) (recv))
            Val::List(vec![
                Val::symbol("def"),
                Val::List(vec![
                    Val::symbol("r"),
                    Val::symbol("src"),
                    Val::symbol("msg"),
                ]),
                Val::List(vec![Val::symbol("recv")]),
            ]),
            // (def resp (try (match ...)))
            Val::List(vec![
                Val::symbol("def"),
                Val::symbol("resp"),
                Val::List(vec![Val::symbol("try"), Val::List(match_form)]),
            ]),
            // (send src (list r resp))
            Val::List(vec![
                Val::symbol("send"),
                Val::symbol("src"),
                Val::List(vec![
                    Val::symbol("list"),
                    Val::symbol("r"),
                    Val::symbol("resp"),
                ]),
            ]),
        ]),
    ]);

    let bc = compile(&ast)?;
    Ok(NativeFnOp::Exec(bc))
}
