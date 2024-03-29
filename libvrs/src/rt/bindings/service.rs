//! Service Bindings
//! See also [super::registry]

use lyric::builtin::cond::is_true;
use lyric::{compile, kwargs, parse, Error, KeywordId, Result, SymbolId};

use crate::rt::program::{Extern, Fiber, Lambda, NativeAsyncFn, NativeFn, NativeFnOp, Val};
use crate::rt::registry::Registration;

/// Binding for register
pub(crate) fn register_fn() -> NativeAsyncFn {
    NativeAsyncFn {
        doc: "(register SVC_NAME [:interface INTERFACE]) - \
              Register caller as SVC_NAME in service registry, optionally providing \
              INTERFACE keyword argument for publishing available interface."
            .to_string(),
        func: |f, args| Box::new(register_impl(f, args)),
    }
}

/// Implementation for (register NAME [:interface INTERFACE_LIST] [:overwrite])
async fn register_impl(fiber: &mut Fiber, args: Vec<Val>) -> Result<Val> {
    let keyword = match args.first() {
        Some(Val::Keyword(k)) => k.clone(),
        _ => {
            return Err(Error::UnexpectedArguments(
                "register expects a keyword argument as first argument".to_string(),
            ))
        }
    };

    let mut reg = Registration::new(keyword);

    if let Some(interface) = kwargs::get(&args[1..], &KeywordId::from("interface")) {
        let symbols = match interface {
            Val::List(ref symbols) => Ok(symbols),
            _ => Err(Error::UnexpectedArguments(
                ":interface keyword argument must be a list".to_string(),
            )),
        }?;
        let symbols = symbols
            .iter()
            .map(|e| match e {
                Val::Symbol(s) => Ok(s),
                _ => Err(Error::UnexpectedArguments(
                    "Forms in :interface list should be symbols".to_string(),
                )),
            })
            .collect::<Result<Vec<_>>>()?;

        let env = fiber.cur_env().lock().unwrap();
        let mut interface = vec![];
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

            if let Some(doc) = lambda.doc {
                reg.docs(sym.clone().to_keyword(), doc);
            }
        }

        reg.interface(interface.clone());
    }

    let overwrite_flag =
        kwargs::flag(&args[1..], &KeywordId::from("overwrite")).unwrap_or(Val::Bool(false));
    if is_true(&overwrite_flag)? {
        reg.overwrite(true);
    }

    let hdl = fiber
        .locals()
        .self_handle
        .as_ref()
        .expect("process should have self handle");

    let registry = fiber
        .locals()
        .registry
        .as_ref()
        .expect("process should have registry handle");

    registry
        .register(reg, hdl.clone())
        .await
        .map_err(|e| Error::Runtime(format!("{e}")))?;

    Ok(Val::keyword("ok"))
}

/// Binding for ls_srv
pub(crate) fn ls_srv_fn() -> NativeAsyncFn {
    NativeAsyncFn {
        doc: "(ls_srv) - Returns a list containing all registered services and exported interface"
            .to_string(),
        func: |f, args| Box::new(ls_srv_impl(f, args)),
    }
}

/// Implementation of (ls_srv)
async fn ls_srv_impl(fiber: &mut Fiber, args: Vec<Val>) -> Result<Val> {
    if !args.is_empty() {
        return Err(Error::UnexpectedArguments(
            "ls_srv expects no arguments".to_string(),
        ));
    }

    let registry = fiber
        .locals()
        .registry
        .as_ref()
        .expect("process should have registry handle");

    let entries = registry
        .all()
        .await
        .map_err(|e| Error::Runtime(format!("{e}")))?;

    let mut entry_values: Vec<_> = vec![];

    for e in entries {
        let val = Val::from(e);
        let name = kwargs::get(val.as_list()?, &KeywordId::from("name")).ok_or(Error::Runtime(
            "service entry did not contain service name".to_string(),
        ))?;
        entry_values.push(name);
        entry_values.push(val.clone());
    }

    Ok(Val::List(entry_values))
}

/// Binding for find_srv
pub(crate) fn find_srv_fn() -> Lambda {
    Lambda {
        doc: Some(
            "(find_srv SVC_NAME) - Returns the process id of SVC_NAME in the service registry. \
              Raises an error if SVC_NAME is not registered."
                .to_string(),
        ),
        params: vec![SymbolId::from("srv_name")],
        code: compile(&parse("(info_srv srv_name :pid)").unwrap().into()).unwrap(),
        parent: None,
    }
}

// TODO: Rust macros for creating Vals - e.g. lambdas
/// Binding for `bind_srv`
pub(crate) fn bind_srv_fn() -> Lambda {
    Lambda {
        doc: Some(
            "(bind_srv SVC_NAME) - Binds to SVC_NAME in service registry, defining new symbols in current process space \
             that communicate to SVC_NAME over message passing."
                .to_string(),
        ),
        params: vec![SymbolId::from("srv_name")],
        code: compile(
            &parse(
                "(map (info_srv srv_name :interface_doc) (lambda (idoc)
                    (def_bind_interface srv_name idoc)))",
            )
            .unwrap()
            .into(),
        )
        .unwrap(),
        parent: None,
    }
}

/// Binding for info_srv
pub(crate) fn info_srv_fn() -> NativeAsyncFn {
    NativeAsyncFn {
        doc: "(info_srv SVC_NAME ATTR) - Returns the attribute ATTR for process registered as SVC_NAME in service registry.".to_string(),
        func: |f, args| Box::new(info_srv_impl(f, args)),
    }
}

/// Implementation for (info_srv NAME ATTR)
async fn info_srv_impl(fiber: &mut Fiber, args: Vec<Val>) -> Result<Val> {
    let (keyword, query) = match &args[..] {
        [Val::Keyword(k), Val::Keyword(q)] => (k, q),
        _ => {
            return Err(Error::UnexpectedArguments(
                "info_srv expects single keyword argument".to_string(),
            ))
        }
    };

    let entry = fiber
        .locals()
        .registry
        .as_ref()
        .expect("no registry for process")
        .lookup(keyword.clone())
        .await
        .map_err(|e| Error::Runtime(format!("{e}")))?
        .ok_or(Error::Runtime(format!("No service found for {keyword}")))?;

    match query.as_str() {
        "pid" => Ok(Val::Extern(Extern::ProcessId(entry.pid()))),
        "interface" => Ok(Val::List(entry.interface().to_vec())),
        "interface_doc" => {
            let mut interface_doc = vec![];
            for i in entry.interface() {
                let kwd = i
                    .as_list()?
                    .first()
                    .ok_or(Error::Runtime(
                        "empty signature in interface list found".to_string(),
                    ))?
                    .as_keyword()?;

                let doc = match entry.doc(kwd) {
                    Some(doc) => doc.to_string(),
                    None => format!("<no documentation for {}>", kwd.clone().to_symbol()),
                };
                interface_doc.push(Val::List(vec![
                    Val::keyword("interface"),
                    i.clone(),
                    Val::keyword("doc"),
                    Val::String(doc),
                ]));
            }

            Ok(Val::List(interface_doc))
        }
        q => Err(Error::UnexpectedArguments(format!(
            "info_srv got unexpected query: {}",
            q
        ))),
    }
}

// TODO: This is a hack to workaround not having macros (yet)
/// Binding for def_bind_interface
pub(crate) fn def_bind_interface() -> NativeFn {
    NativeFn {
        doc: "(def_bind_interface SVC_NAME INTERFACE_DOC) - Runtime internal use only. Shim for service bindings".to_string(),
        func: |f, args| {
            let (svc_name, interface_doc) =
                match args {
                    [Val::Keyword(svc_name), Val::List(idoc)] => (svc_name, idoc),
                    _ => return Err(Error::UnexpectedArguments(
                        "def_bind_interface expects a keyword for service and interface doc list it exposes"
                            .to_string(),
                    )),
                };

            let interface = kwargs::get(interface_doc, &KeywordId::from("interface"))
                .ok_or(Error::UnexpectedArguments(
                    "interface doc should have interface kwarg".to_string(),
                ))?.to_list()?;
            let doc = kwargs::get(interface_doc, &KeywordId::from("doc"))
                .ok_or(Error::UnexpectedArguments(
                    "interface doc should have doc kwarg".to_string(),
                ))?
                .as_string()?
                .clone();

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
                        "def_bind_interface expects a symbols after first keyword-argument"
                            .to_string(),
                    )),
                })
                .collect::<Result<Vec<_>>>()?;

            let mut env = f.global_env().lock().unwrap();
            let sym = msg_name.clone().to_symbol();
            env.define(
                sym,
                Val::Lambda(lambda_stub_for_interface(
                    svc_name, arg_syms, msg_name, args, doc
                )),
            );

            Ok(NativeFnOp::Return(Val::List(interface.to_vec())))
        },
    }
}

// TODO: Define as lisp macro
/// Implementation of spawn_srv
pub(crate) fn spawn_srv_fn() -> NativeFn {
    NativeFn {
        doc: "(spawn_srv SVC_NAME [:interface INTERFACE]) - Spawn a separate process as service registered as SVC_NAME, \
              optionally exporting interface INTERFACE.".to_string(),
        func: spawn_srv_impl,
    }
}

/// Implementation for (spawn_srv) that matches (srv)'s signature
fn spawn_srv_impl(_f: &mut Fiber, args: &[Val]) -> Result<NativeFnOp> {
    // Expand
    //     (spawn_srv :SRV_NAME :interface '(sym_a sym_b))
    // Into
    //     (spawn (lambda () (begin
    //            (try (kill (find_srv :SRV_NAME)))
    //            (srv :SRV_NAME :interface '(sym_a sym_b)))))

    let mut srv = vec![Val::symbol("srv")];
    srv.push(args[0].clone());

    if let Some(interfaces) = kwargs::get(args, &KeywordId::from("interface")) {
        srv.push(Val::keyword("interface"));
        srv.push(Val::List(vec![Val::symbol("quote"), interfaces.clone()]));
    }

    let kill_srv = Val::from_expr(&format!("(try (kill (find_srv {})))", args[0].clone())).unwrap();

    let ast = Val::List(vec![
        Val::symbol("spawn"),
        Val::List(vec![
            Val::symbol("lambda"),
            Val::List(vec![]),
            Val::List(vec![Val::symbol("begin"), kill_srv, Val::List(srv)]),
        ]),
    ]);

    let bc = compile(&ast)?;
    Ok(NativeFnOp::Exec(bc))
}

/// Binding for `srv`
pub(crate) fn srv_fn() -> NativeFn {
    NativeFn {
        doc: "(srv SVC_NAME [:interface INTERFACE]) - Register current process as SVC_NAME, \
              optionally exporting interface INTERFACE. This function blocks until service exits."
            .to_string(),
        func: srv_impl,
    }
}

// TODO: Define as lisp macro
fn srv_impl(f: &mut Fiber, args: &[Val]) -> Result<NativeFnOp> {
    // Expand
    //     (srv :SRV_NAME :interface '(sym_a sym_b))
    // to
    //     (begin
    //         (register :launcher :overwrite :interface '(sym_a sym_b))
    //         (loop
    //             (def (r src msg) (recv))
    //             (def resp
    //                 (try (match msg
    //                     ((:sym_a arg1 arg2) (sym_a arg1 arg2))
    //                     ((:sym_b) (sym_b))
    //                     (_ '(:err "Unrecognized message")))))
    //             (send src (list r resp))))

    let name = args.first().ok_or(Error::UnexpectedArguments(
        "First argument must be a value used to identify service".to_string(),
    ))?;

    let interface = kwargs::get(args, &KeywordId::from("interface")).ok_or(
        Error::UnexpectedArguments("Missing :interface keyword argument".to_string()),
    )?;
    let symbols = match interface {
        Val::List(ref symbols) => Ok(symbols),
        _ => Err(Error::UnexpectedArguments(
            ":interface keyword argument must be a list".to_string(),
        )),
    }?;
    let symbols = symbols
        .iter()
        .map(|e| match e {
            Val::Symbol(s) => Ok(s),
            _ => Err(Error::UnexpectedArguments(
                "Forms in :interface list should be symbols".to_string(),
            )),
        })
        .collect::<Result<Vec<_>>>()?;

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
        name.clone(),
        Val::keyword("overwrite"),
        Val::keyword("interface"),
        Val::List(vec![Val::symbol("quote"), interface]),
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
    doc: String,
) -> Lambda {
    // TODO: Need to do this hack since there's no splice in lists atm
    let msg = [Val::symbol("list"), Val::Keyword(msg_name.clone())]
        .into_iter()
        .chain(msg_args.iter().cloned())
        .collect::<Vec<_>>();
    let ast =
        parse(format!(r#"(call (find_srv {}) {})"#, srv_name, Val::List(msg)).as_str()).unwrap();
    let code = compile(&ast.into()).unwrap();
    Lambda {
        doc: Some(doc),
        params,
        code,
        parent: None,
    }
}

#[cfg(test)]
mod tests {
    use lyric::Inst;

    use crate::rt::bindings::service::lambda_stub_for_interface;

    use super::*;

    #[test]
    fn lambda_interface_empty() {
        let lambda = Lambda {
            doc: None,
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
            doc: None,
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
            doc: None,
            params: vec![],
            code: vec![Inst::PushConst(Val::Nil)],
            parent: None,
        };

        assert_eq!(lambda_call(&SymbolId::from("hello"), &lambda), v("(hello)"));
    }

    #[test]
    fn lambda_call_nonempty() {
        let lambda = Lambda {
            doc: None,
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
            let lambda = lambda_stub_for_interface(
                &srv_name,
                vec![],
                &KeywordId::from("get_items"),
                &[],
                String::new(),
            );
            assert_eq!(
                lambda,
                Lambda {
                    doc: None,
                    params: vec![],
                    code: compile(&v(r#"
                        (call (find_srv :launcher) (list :get_items))
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
                String::new(),
            );
            assert_eq!(
                lambda,
                Lambda {
                    doc: None,
                    params: vec![SymbolId::from("title"), SymbolId::from("cmd")],
                    code: compile(&v(r#"
                        (call (find_srv :launcher) (list :add_item title cmd))
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
