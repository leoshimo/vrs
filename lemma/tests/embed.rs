//! Tests for embedding in host application

// TODO: Revive once Fiber API is stable

// use assert_matches::assert_matches;
// use lemma::{Error, Fiber, Inst, State, Val};

// #[test]
// fn fiber_new() {
//     let f = Fiber::from_val(&Val::string("hello world")).expect("should be created");
//     assert_eq!(f.state(), State::New);
// }

// #[test]
// fn fiber_simple() {
//     let mut f = Fiber::from_val(&Val::string("hello world")).expect("should be created");
//     assert_eq!(f.state(), State::New);
//     f.start();
//     assert_eq!(f.state(), Status::Completed(Ok(Val::string("hello world"))));
// }

// #[test]
// fn fiber_invalid_expr() {
//     assert_matches!(
//         Fiber::from_expr("- jibberish )("),
//         Err(Error::FailedToLex(_))
//     );
// }

// #[test]
// fn fiber_empty_bytecode() {
//     // TODO: Propagate as error instead?
//     let result = std::panic::catch_unwind(|| {
//         let mut f = Fiber::from_bytecode(vec![]);
//         f.start();
//     });
//     assert_matches!(result, Err(_), "Executing empty bytecode panics");
// }

// #[test]
// fn fiber_invalid_bytecode() {
//     // TODO: Propagate as error instead?
//     let mut f = Fiber::from_bytecode(vec![Inst::PopTop, Inst::PopTop, Inst::PopTop]);
//     f.start();
//     assert_matches!(
//         *f.status(),
//         Status::Completed(Err(Error::UnexpectedStack(_)))
//     );
// }

// #[test]
// fn fiber_yielding() {
//     let mut f = Fiber::from_bytecode(vec![]); // TODO: Fill instructions

//     // TODO: Does match f.status() loop even work!? with mutable manipulations of f?
//     f.start()
// }

// TODO: Test error propagation when fiber is already running
