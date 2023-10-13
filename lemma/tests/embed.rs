//! Tests for embedding in host application
use assert_matches::assert_matches;
use lemma::{fiber, Error, Fiber, Inst, Status, Val};

#[test]
fn fiber_new() {
    let f = Fiber::from_val(&Val::string("hello world")).expect("should be created");
    assert_eq!(*f.status(), Status::New);
}

#[test]
fn fiber_simple() {
    let mut f = Fiber::from_val(&Val::string("hello world")).expect("should be created");
    assert_eq!(*f.status(), Status::New);

    let status = fiber::start(&mut f).expect("should start");
    assert_eq!(*status, Status::Completed(Ok(Val::string("hello world"))));
    assert_eq!(
        *f.status(),
        Status::Completed(Ok(Val::string("hello world")))
    );
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
    // TODO: Propagate as error instead?
    let result = std::panic::catch_unwind(|| {
        let mut f = Fiber::from_bytecode(vec![]);
        fiber::start(&mut f).expect("should start");
    });
    assert_matches!(result, Err(_), "Executing empty bytecode panics");
}

#[test]
fn fiber_invalid_bytecode() {
    // TODO: Propagate as error instead?
    let mut f = Fiber::from_bytecode(vec![Inst::PopTop, Inst::PopTop, Inst::PopTop]);
    let s = fiber::start(&mut f).expect("should start");
    assert_matches!(*s, Status::Completed(Err(Error::UnexpectedStack(_))));
    assert_matches!(
        *f.status(),
        Status::Completed(Err(Error::UnexpectedStack(_)))
    );
}

#[test]
fn fiber_yielding() {}

// TODO: Test error propagation when fiber is already running
