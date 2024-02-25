//! Procedures that drive a top-level [Fiber] to completion through it's coroutine interface.
//!
//! Lyric's implementation avoids Async Rust in core implementation - only
//! exception is `NativeFnAsync` calls.  This is to allow `lyric` to be embedded
//! inside applications without async runtimes, and make development of `lyric`
//! simpler.
//!
//! Procedures in this module wrap Fiber execution in several ways that may be
//! ergonomic for hosting application, e.g. via spawning thread-per-fiber, or
//! via future to take advantage of async IO if it is available.

use crate::{Error, Extern, Fiber, Locals, Result, Signal, Val};

/// Run the fiber to completion as a Future
pub async fn run<T, L>(f: &mut Fiber<T, L>) -> Result<Val<T, L>>
where
    T: Extern,
    L: Locals,
{
    let mut res = f.start()?;
    loop {
        match res {
            Signal::Done(v) => return Ok(v),
            Signal::Yield(_) => return Err(Error::UnexpectedTopLevelYield),
            Signal::Await(call) => {
                // TODO: Should errors in fut properly update `Fiber::state`?
                // TODO: Jiggle code between fiber::run and run::run
                // TODO(bug): NativeAsyncFn do not respect error catching scope, e.g. `(try (exec "jibberish"))` terminates proc
                let poll_res = call.apply(f).await;
                res = f.resume(poll_res)?;
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::{Env, NativeAsyncFn, SymbolId};
    use assert_matches::assert_matches;
    use tokio::task::yield_now;

    use super::*;
    use void::Void;

    type Fiber = crate::Fiber<Void, ()>;

    #[tokio::test]
    async fn run_returns_fiber_result() {
        let prog = r#"(begin
            (defn inc (x)
                (+ x 1))
            (def x 39)
            (set x (inc x))
            (set x (inc x))
            (set x (inc x))
        )"#;
        let mut f = Fiber::from_expr(prog, Env::standard(), ()).unwrap();
        assert_eq!(run(&mut f).await.unwrap(), Val::Int(42));
    }

    #[tokio::test]
    async fn top_level_yield_is_error() {
        let prog = r#"(yield 10)"#;
        let mut f = Fiber::from_expr(prog, Env::standard(), ()).unwrap();
        assert_matches!(run(&mut f).await, Err(Error::UnexpectedTopLevelYield));
    }

    #[tokio::test]
    async fn run_hits_error() {
        let prog = r#"(begin
            (defn inc (x)
                (undefined_function)
                (+ x 1))
            (def x 39)
            (set x (inc x))
        )"#;
        let mut f = Fiber::from_expr(prog, Env::standard(), ()).unwrap();
        assert_matches!(run(&mut f).await, Err(Error::UndefinedSymbol(_)));
    }

    #[tokio::test]
    async fn await_then_done() {
        let prog = r#"(async_call)"#;

        let mut env = Env::standard();
        env.bind_native_async(
            SymbolId::from("async_call"),
            NativeAsyncFn {
                func: |_, _| {
                    Box::new(async {
                        yield_now().await;
                        Ok(Val::string("from async"))
                    })
                },
            },
        );

        let mut f = Fiber::from_expr(prog, env, ()).unwrap();
        assert_eq!(run(&mut f).await.unwrap(), Val::string("from async"));
    }

    #[tokio::test]
    async fn await_nested() {
        let prog = r#"(async_inc (async_inc (async_inc 39)))"#;

        let mut env = Env::standard();
        env.bind_native_async(
            SymbolId::from("async_inc"),
            NativeAsyncFn {
                func: |_, args| {
                    let num = match args[..] {
                        [Val::Int(n)] => n,
                        _ => panic!(),
                    };
                    Box::new(async move {
                        yield_now().await;
                        Ok(Val::Int(num + 1))
                    })
                },
            },
        );

        let mut f = Fiber::from_expr(prog, env, ()).unwrap();
        assert_eq!(run(&mut f).await.unwrap(), Val::Int(42));
    }

    #[tokio::test]
    async fn error_during_await() {
        let prog = r#"(async_err 1 2 3)"#;

        let mut env = Env::standard();
        env.bind_native_async(
            SymbolId::from("async_err"),
            NativeAsyncFn {
                func: |_, args| {
                    Box::new(async move {
                        yield_now().await;
                        Err(Error::UnexpectedArguments(format!(
                            "Unexpected arguments - {}",
                            Val::List(args)
                        )))
                    })
                },
            },
        );

        let mut f = Fiber::from_expr(prog, env, ()).unwrap();
        assert_matches!(run(&mut f).await, Err(Error::UnexpectedArguments(s)) if s == "Unexpected arguments - (1 2 3)");
    }

    #[tokio::test]
    async fn error_during_nested_await() {
        let prog = r#"(async_inc (async_inc (async_inc (async_inc (async_inc (async_inc 0))))))"#;

        let mut env = Env::standard();
        env.bind_native_async(
            SymbolId::from("async_inc"),
            NativeAsyncFn {
                func: |_, args| {
                    let num = match args[..] {
                        [Val::Int(n)] => n,
                        _ => panic!(),
                    };
                    Box::new(async move {
                        yield_now().await;
                        if num == 3 {
                            Err(Error::UnexpectedArguments(
                                "Cannot be called with argument 3".to_string(),
                            ))
                        } else {
                            Ok(Val::Int(num + 1))
                        }
                    })
                },
            },
        );

        let mut f = Fiber::from_expr(prog, env, ()).unwrap();
        assert_matches!(run(&mut f).await, Err(Error::UnexpectedArguments(s)) if s == "Cannot be called with argument 3");
    }

    #[tokio::test]
    async fn run_is_send() {
        fn require_send<T: Send>(_t: &T) {}

        let prog = r#"(def x 4)"#;
        let mut f = Fiber::from_expr(prog, Env::standard(), ()).unwrap();
        require_send(&run(&mut f));
    }
}
