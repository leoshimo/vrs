//! Tests for embedding in host application

use assert_matches::assert_matches;
use lemma::{Error, Fiber, FiberState, Inst, Val};
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

// #[test]
// fn fiber_yielding() {
//     let mut f = Fiber::from_bytecode(vec![]); // TODO: Fill instructions

//     // TODO: Does match f.status() loop even work!? with mutable manipulations of f?
//     f.start()
// }

// // TODO: Test error propagation when fiber is already running
