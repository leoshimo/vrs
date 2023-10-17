//! Tests for embedding in host application

use assert_matches::assert_matches;
use lemma::{parse, Error, Fiber, FiberState, Inst, NativeFn, NativeFnVal, SymbolId, Val};
use FiberState::*;

#[test]
fn fiber_simple() {
    let mut f = Fiber::from_val(&Val::string("hello world")).expect("should be created");
    assert_eq!(f.resume(), Ok(Done(Val::string("hello world"))));
}

#[test]
fn fiber_invalid_expr() {
    assert_matches!(
        Fiber::from_expr("- jibberish )("),
        Err(Error::FailedToLex(_))
    );
}

#[test]
fn fiber_empty_bytecode() {
    let mut f = Fiber::from_bytecode(vec![]);
    assert_matches!(
        f.resume(),
        Err(Error::UnexpectedStack(_)),
        "Executing empty bytecode panics, since there is nothing to return"
    );
}

#[test]
fn fiber_invalid_bytecode() {
    let mut f = Fiber::from_bytecode(vec![Inst::PopTop, Inst::PopTop, Inst::PopTop]);
    assert_matches!(f.resume(), Err(Error::UnexpectedStack(_)));
}

#[test]
fn fiber_yielding() {
    // An infinitely increasing counter increasing by one each iteration
    let prog = r#"
        (begin 
            (def x 0)
            (defn yielding_add ()
                (yield x)
                (set x (+ x 1))
                (yielding_add))
            (yielding_add)
        )
    "#;
    let mut f = Fiber::from_expr(prog).unwrap();

    assert_eq!(f.resume().unwrap(), Yield(Val::Int(0)));
    assert_eq!(f.resume_from_yield(Val::Nil).unwrap(), Yield(Val::Int(1)));
    assert_eq!(f.resume_from_yield(Val::Nil).unwrap(), Yield(Val::Int(2)));
    assert_eq!(f.resume_from_yield(Val::Nil).unwrap(), Yield(Val::Int(3)));
    assert_eq!(f.resume_from_yield(Val::Nil).unwrap(), Yield(Val::Int(4)));
    assert_eq!(f.resume_from_yield(Val::Nil).unwrap(), Yield(Val::Int(5)));

    // TODO: Use fiber_yielding as test case for tail-call optimization (shouldn't grow number of CallFrames)
}

#[test]
fn fiber_yielding_by_arg() {
    // An infinitely increasing counter increasing by yield-ed value
    let prog = r#"
        (begin 
            (def x 0)
            (defn yielding_add ()
                (set x (+ x (yield x)))
                (yielding_add))
            (yielding_add)
        )
    "#;
    let mut f = Fiber::from_expr(prog).unwrap();

    assert_eq!(f.resume().unwrap(), Yield(Val::Int(0)));
    assert_eq!(
        f.resume_from_yield(Val::Int(1)).unwrap(),
        Yield(Val::Int(1))
    );
    assert_eq!(
        f.resume_from_yield(Val::Int(2)).unwrap(),
        Yield(Val::Int(3))
    );
    assert_eq!(
        f.resume_from_yield(Val::Int(3)).unwrap(),
        Yield(Val::Int(6))
    );
    assert_eq!(
        f.resume_from_yield(Val::Int(4)).unwrap(),
        Yield(Val::Int(10))
    );
    assert_eq!(
        f.resume_from_yield(Val::Int(5)).unwrap(),
        Yield(Val::Int(15))
    );
}

#[test]
fn fiber_yielding_native_binding() {
    let mut f = Fiber::from_expr("(echo_yield :one :two)").unwrap();
    f.bind(NativeFn {
        symbol: SymbolId::from("echo_yield"),
        func: |_, x| Ok(NativeFnVal::Yield(Val::Signal(42, x.to_vec()))),
    });

    assert_eq!(
        f.resume().unwrap(),
        Yield(Val::Signal(
            42,
            vec![Val::keyword("one"), Val::keyword("two"),]
        ))
    );
    assert_eq!(
        f.resume_from_yield(Val::string("Hello world")).unwrap(),
        Done(Val::string("Hello world")),
    );
}

#[test]
fn fiber_looping_yield() {
    let prog = r#"
        (begin
            (def x 0)
            (loop (set x (+ x (yield x)))))
    "#;

    let mut f = Fiber::from_expr(prog).unwrap();

    assert_eq!(f.resume().unwrap(), Yield(Val::Int(0)));
    assert_eq!(
        f.resume_from_yield(Val::Int(1)).unwrap(),
        Yield(Val::Int(1))
    );
    assert_eq!(
        f.resume_from_yield(Val::Int(2)).unwrap(),
        Yield(Val::Int(3))
    );
    assert_eq!(
        f.resume_from_yield(Val::Int(3)).unwrap(),
        Yield(Val::Int(6))
    );
}

#[test]
fn fiber_conn_recv_peval_sim() {
    // program representing client REPL loop
    let prog = r#"
        (loop (send_conn (peval (recv_conn))))
    "#;

    let mut f = Fiber::from_expr(prog).unwrap();
    f.bind(NativeFn {
        symbol: SymbolId::from("recv_conn"),
        func: |_, _| {
            Ok(NativeFnVal::Yield(Val::signal(
                0,
                vec![Val::keyword("recv_conn")],
            )))
        },
    });
    f.bind(NativeFn {
        symbol: SymbolId::from("send_conn"),
        func: |_, args| {
            Ok(NativeFnVal::Yield(Val::signal(
                1,
                std::iter::once(Val::keyword("send_conn"))
                    .chain(args.iter().cloned())
                    .collect(),
            )))
        },
    });
    f.bind(NativeFn {
        symbol: SymbolId::from("peval"),
        func: |f, args| {
            let v = match args {
                [v] => v,
                _ => {
                    return Err(Error::InvalidExpression(
                        "peval expects one argument".to_string(),
                    ))
                }
            };
            let mut f = Fiber::from_val(v)?.with_env(f.env());
            match f.resume() {
                Ok(FiberState::Done(v)) => Ok(NativeFnVal::Return(v)),
                Ok(FiberState::Yield(v)) => Ok(NativeFnVal::Yield(v)),
                Err(e) => Ok(NativeFnVal::Return(Val::Error(e))),
            }
        },
    });

    assert_eq!(
        f.resume().unwrap(),
        Yield(Val::signal(0, vec![Val::keyword("recv_conn")])),
        "Should yield for recv_conn"
    );

    assert_eq!(
        f.resume_from_yield(parse("(def x (+ 1 2))").unwrap().into())
            .unwrap(),
        Yield(Val::signal(1, vec![Val::keyword("send_conn"), Val::Int(3)])),
        "Should receive send_conn signal w/ eval-ed expr"
    );
    assert_eq!(
        f.resume_from_yield(Val::Nil).unwrap(),
        Yield(Val::signal(0, vec![Val::keyword("recv_conn")])),
        "Should yield for recv_conn again"
    );

    assert_eq!(
        f.resume_from_yield(parse("x").unwrap().into()).unwrap(),
        Yield(Val::signal(1, vec![Val::keyword("send_conn"), Val::Int(3)])),
        "Should receive send_conn signal w/ eval-ed expr"
    );
    assert_eq!(
        f.resume_from_yield(Val::Nil).unwrap(),
        Yield(Val::signal(0, vec![Val::keyword("recv_conn")])),
        "Should yield for recv_conn again"
    );

    assert_eq!(
        f.resume_from_yield(Val::symbol("jibberish")).unwrap(),
        Yield(Val::signal(
            1,
            vec![
                Val::keyword("send_conn"),
                Val::Error(Error::UndefinedSymbol(SymbolId::from("jibberish")))
            ]
        )),
        "Error should return error as a value via pcall"
    );
    assert_eq!(
        f.resume_from_yield(Val::Nil).unwrap(),
        Yield(Val::signal(0, vec![Val::keyword("recv_conn")])),
        "Should yield for recv_conn again"
    );

    assert_eq!(
        f.resume_from_yield(parse("(set x (+ x x))").unwrap().into())
            .unwrap(),
        Yield(Val::signal(1, vec![Val::keyword("send_conn"), Val::Int(6)])),
        "Environment should be preserved after error"
    );
}
