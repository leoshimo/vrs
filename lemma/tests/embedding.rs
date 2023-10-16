//! Tests for embedding in host application

use assert_matches::assert_matches;
use lemma::{Error, Fiber, FiberState, Inst, NativeFn, NativeFnVal, SymbolId, Val};
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
    f.bind(NativeFn {
        symbol: SymbolId::from("+"),
        func: |x| match x {
            [Val::Int(a), Val::Int(b)] => Ok(NativeFnVal::Return(Val::Int(a + b))),
            _ => panic!("only supports ints"),
        },
    });

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
    f.bind(NativeFn {
        symbol: SymbolId::from("+"),
        func: |x| match x {
            [Val::Int(a), Val::Int(b)] => Ok(NativeFnVal::Return(Val::Int(a + b))),
            _ => panic!("only supports ints"),
        },
    });

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
        func: |x| Ok(NativeFnVal::Yield(Val::Signal(42, x.to_vec()))),
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

// TODO: Test error propagation when fiber is already running
