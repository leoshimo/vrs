//! Service Bindings
//! See also [super::registry]
use lyric::{compile, kwargs, Error, KeywordId, Result, SymbolId};

use crate::rt::proc_io::IOCmd;
use crate::rt::program::{Extern, Fiber, Lambda, NativeFn, NativeFnOp, Val};
use crate::rt::registry::Registration;

/// Binding for register
pub(crate) fn register_fn() -> NativeFn {
    NativeFn {
        func: |_, args| {
            let keyword = match args.first() {
                Some(Val::Keyword(k)) => k.clone(),
                _ => {
                    return Err(Error::UnexpectedArguments(
                        "register expects a keyword argument as first argument".to_string(),
                    ))
                }
            };

            let mut reg = Registration::new(keyword);
            match kwargs::get(&args[1..], &KeywordId::from("exports")) {
                Some(Val::List(exports)) => {
                    reg.exports(exports);
                }
                Some(val) => {
                    return Err(Error::UnexpectedArguments(format!(
                        ":exports must be a list - got {}",
                        val
                    )))
                }
                None => (),
            }

            Ok(NativeFnOp::Yield(Val::Extern(Extern::IOCmd(Box::new(
                IOCmd::RegisterAsService(reg),
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
    {
        let env = f.cur_env().lock().unwrap();

        for sym in exports {
            let val = env.get(&sym).ok_or(Error::InvalidExpression(format!(
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
            match_form.push(Val::List(vec![
                lambda_pattern(&sym, &lambda),
                lambda_call(&sym, &lambda),
            ]));
        }
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

/// Generates pattern for calling exported lambda
fn lambda_pattern(symbol: &SymbolId, lambda: &Lambda) -> Val {
    Val::List(
        std::iter::once(Val::Keyword(symbol.clone().to_keyword()))
            .chain(lambda.params.iter().map(|v| Val::Symbol(v.clone())))
            .collect::<Vec<_>>(),
    )
}

/// Generates function call expression compatible with [lambda_pattern]
fn lambda_call(symbol: &SymbolId, lambda: &Lambda) -> Val {
    //
    Val::List(
        std::iter::once(Val::Symbol(symbol.clone()))
            .chain(lambda.params.iter().map(|v| Val::Symbol(v.clone())))
            .collect::<Vec<_>>(),
    )
}

#[cfg(test)]
mod tests {
    use lyric::Inst;

    use super::*;

    #[test]
    fn lambda_pattern_empty() {
        let lambda = Lambda {
            params: vec![],
            code: vec![Inst::PushConst(Val::Nil)],
            parent: None,
        };

        assert_eq!(
            lambda_pattern(&SymbolId::from("hello"), &lambda),
            v("(:hello)")
        );
    }

    #[test]
    fn lambda_pattern_nonempty() {
        let lambda = Lambda {
            params: vec![SymbolId::from("arg1"), SymbolId::from("arg2")],
            code: vec![Inst::PushConst(Val::Nil)],
            parent: None,
        };

        assert_eq!(
            lambda_pattern(&SymbolId::from("hello"), &lambda),
            v("(:hello arg1 arg2)")
        );
    }

    #[test]
    fn lambda_call_empty() {
        let lambda = Lambda {
            params: vec![],
            code: vec![Inst::PushConst(Val::Nil)],
            parent: None,
        };

        assert_eq!(lambda_call(&SymbolId::from("hello"), &lambda), v("(hello)"));
    }

    #[test]
    fn lambda_call_nonempty() {
        let lambda = Lambda {
            params: vec![SymbolId::from("arg1"), SymbolId::from("arg2")],
            code: vec![Inst::PushConst(Val::Nil)],
            parent: None,
        };

        assert_eq!(
            lambda_call(&SymbolId::from("hello"), &lambda),
            v("(hello arg1 arg2)")
        );
    }

    fn v(expr: &str) -> Val {
        lyric::parse(expr).unwrap().into()
    }
}
