//! Service Bindings
//! See also [super::registry]

use lyric::{compile, kwargs, parse, Error, KeywordId, Result, SymbolId};

use crate::rt::proc_io::{IOCmd, ServiceQuery};
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
            match kwargs::get(&args[1..], &KeywordId::from("interface")) {
                Some(Val::List(interface)) => {
                    reg.interface(interface);
                }
                Some(val) => {
                    return Err(Error::UnexpectedArguments(format!(
                        ":interface must be a list - got {}",
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
                    "ls-srv expects no arguments".to_string(),
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
                IOCmd::QueryService(keyword, ServiceQuery::Pid),
            )))))
        },
    }
}

/// Binding for `srv`
pub(crate) fn srv_fn() -> NativeFn {
    NativeFn { func: srv }
}

// TODO: Rust macros for creating Vals - e.g. lambdas
/// Binding for `bind-srv`
pub(crate) fn bind_srv_fn() -> Lambda {
    Lambda {
        params: vec![SymbolId::from("srv_name")],
        code: compile(
            &parse(
                "(map (info-srv srv_name :interface) (lambda (i) (def-bind-interface srv_name i)))",
            )
            .unwrap()
            .into(),
        )
        .unwrap(),
        parent: None,
    }
}

/// Binding for info-srv
pub(crate) fn info_srv_fn() -> NativeFn {
    NativeFn {
        func: |_, args| {
            let (keyword, query) = match args {
                [Val::Keyword(k), Val::Keyword(q)] => (k, q),
                _ => {
                    return Err(Error::UnexpectedArguments(
                        "info-srv expects single keyword argument".to_string(),
                    ))
                }
            };

            let query = match query.as_str() {
                "pid" => ServiceQuery::Pid,
                "interface" => ServiceQuery::Interface,
                q => {
                    return Err(Error::UnexpectedArguments(format!(
                        "info-srv got unexpected query: {}",
                        q
                    )))
                }
            };

            Ok(NativeFnOp::Yield(Val::Extern(Extern::IOCmd(Box::new(
                IOCmd::QueryService(keyword.clone(), query),
            )))))
        },
    }
}

// TODO: This is a hack to workaround not having macros (yet)
/// Binding for def-bind-interface
pub(crate) fn def_bind_interface() -> NativeFn {
    NativeFn {
        func: |f, args| {
            let (svc_name, interface) =
                match args {
                    [Val::Keyword(svc_name), Val::List(interface)] => (svc_name, interface),
                    _ => return Err(Error::UnexpectedArguments(
                        "def-bind-interface expects a keyword for service and interface it exposes"
                            .to_string(),
                    )),
                };

            let (msg_name, args) = interface.split_first().ok_or(Error::UnexpectedArguments(
                "interface list must contain at least one item".to_string(),
            ))?;

            let msg_name = match msg_name {
                Val::Keyword(k) => Ok(k),
                v => Err(Error::UnexpectedArguments(format!(
                    "first element of interface item should be keyword - got {}",
                    v
                ))),
            }?;

            let arg_syms = args
                .iter()
                .cloned()
                .map(|m| match m {
                    Val::Symbol(sym) => Ok(sym),
                    _ => Err(Error::UnexpectedArguments(
                        "def-bind-interface expects a symbols after first keyword-argument"
                            .to_string(),
                    )),
                })
                .collect::<Result<Vec<_>>>()?;

            let mut env = f.global_env().lock().unwrap();
            let sym = msg_name.clone().to_symbol();
            env.define(
                sym,
                Val::Lambda(lambda_stub_for_interface(
                    svc_name, arg_syms, msg_name, args,
                )),
            );

            Ok(NativeFnOp::Return(Val::List(interface.to_vec())))
        },
    }
}

// TODO: Define as lisp macro
fn srv(f: &mut Fiber, args: &[Val]) -> Result<NativeFnOp> {
    // Expand
    //     (srv :name :SRV_NAME :exports '(sym_a sym_b))
    // to
    //     (begin
    //         (register :launcher :exports '(sym_a sym_b))
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
    let symbols = match exports {
        Val::List(ref symbols) => Ok(symbols),
        _ => Err(Error::UnexpectedArguments(
            ":exports keyword argument must be a list".to_string(),
        )),
    }?;
    let symbols = symbols
        .iter()
        .map(|e| match e {
            Val::Symbol(s) => Ok(s),
            _ => Err(Error::UnexpectedArguments(
                "Forms in :exports list should be symbols".to_string(),
            )),
        })
        .collect::<Result<Vec<_>>>()?;

    let mut interface = vec![];
    let mut match_form = vec![Val::symbol("match"), Val::symbol("msg")];

    {
        let env = f.cur_env().lock().unwrap();

        for sym in symbols {
            let val = env.get(sym).ok_or(Error::InvalidExpression(format!(
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
            let pattern = lambda_interface(sym, &lambda);
            interface.push(pattern.clone());
            match_form.push(Val::List(vec![pattern, lambda_call(sym, &lambda)]));
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

    let register_form = Val::List(vec![
        Val::symbol("register"),
        name,
        Val::keyword("interface"),
        Val::List(vec![Val::symbol("quote"), Val::List(interface)]),
    ]);

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

/// Generates interface for calling exported lambda
fn lambda_interface(symbol: &SymbolId, lambda: &Lambda) -> Val {
    Val::List(
        std::iter::once(Val::Keyword(symbol.clone().to_keyword()))
            .chain(lambda.params.iter().map(|v| Val::Symbol(v.clone())))
            .collect::<Vec<_>>(),
    )
}

/// Generates function call expression compatible with [lambda_interface]
fn lambda_call(symbol: &SymbolId, lambda: &Lambda) -> Val {
    //
    Val::List(
        std::iter::once(Val::Symbol(symbol.clone()))
            .chain(lambda.params.iter().map(|v| Val::Symbol(v.clone())))
            .collect::<Vec<_>>(),
    )
}

/// Given a [lambda_interface] [Val], turns it into client-side =Lambda= definition
fn lambda_stub_for_interface(
    srv_name: &KeywordId,
    params: Vec<SymbolId>,
    msg_name: &KeywordId,
    msg_args: &[Val],
) -> Lambda {
    // TODO: Need to do this hack since there's no splice in lists atm
    let msg = [Val::symbol("list"), Val::Keyword(msg_name.clone())]
        .into_iter()
        .chain(msg_args.iter().cloned())
        .collect::<Vec<_>>();
    let ast =
        parse(format!(r#"(call (find-srv {}) {})"#, srv_name, Val::List(msg)).as_str()).unwrap();
    let code = compile(&ast.into()).unwrap();
    Lambda {
        params,
        code,
        parent: None,
    }
}

#[cfg(test)]
mod tests {
    use lyric::Inst;

    use crate::rt::binding::service::lambda_stub_for_interface;

    use super::*;

    #[test]
    fn lambda_interface_empty() {
        let lambda = Lambda {
            params: vec![],
            code: vec![Inst::PushConst(Val::Nil)],
            parent: None,
        };

        assert_eq!(
            lambda_interface(&SymbolId::from("hello"), &lambda),
            v("(:hello)")
        );
    }

    #[test]
    fn lambda_interface_nonempty() {
        let lambda = Lambda {
            params: vec![SymbolId::from("arg1"), SymbolId::from("arg2")],
            code: vec![Inst::PushConst(Val::Nil)],
            parent: None,
        };

        assert_eq!(
            lambda_interface(&SymbolId::from("hello"), &lambda),
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

    #[test]
    fn stub_for_interface() {
        {
            let srv_name = KeywordId::from("launcher");
            let lambda =
                lambda_stub_for_interface(&srv_name, vec![], &KeywordId::from("get_items"), &[]);
            assert_eq!(
                lambda,
                Lambda {
                    params: vec![],
                    code: compile(&v(r#"
                        (call (find-srv :launcher) (list :get_items))
                        "#))
                    .unwrap(),
                    parent: None
                }
            )
        }
        {
            let srv_name = KeywordId::from("launcher");
            let lambda = lambda_stub_for_interface(
                &srv_name,
                vec![SymbolId::from("title"), SymbolId::from("cmd")],
                &KeywordId::from("add_item"),
                &[Val::symbol("title"), Val::symbol("cmd")],
            );
            assert_eq!(
                lambda,
                Lambda {
                    params: vec![SymbolId::from("title"), SymbolId::from("cmd")],
                    code: compile(&v(r#"
                        (call (find-srv :launcher) (list :add_item title cmd))
                        "#))
                    .unwrap(),
                    parent: None,
                }
            )
        }
    }

    fn v(expr: &str) -> Val {
        lyric::parse(expr).unwrap().into()
    }
}
