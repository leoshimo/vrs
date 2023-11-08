//! Service Bindings
//! See also [super::registry]
use lyric::{compile, Error, Result};

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
fn srv(fiber: &mut Fiber, args: &[Val]) -> Result<NativeFnOp> {
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

    let srv_name = Val::keyword("todo");
    let exports = Val::List(vec![]);

    let register_form = Val::List(vec![Val::symbol("register"), srv_name]);

    let mut match_form = vec![Val::symbol("match"), Val::symbol("msg")];
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
