#![allow(dead_code, unused_imports)]
//! A running lyric program
//! [lyric::Process] drives execution of a [lyric::Fiber]
use super::{Env, Inst};
use crate::{
    builtin::cond::is_true, compile, parse, Bytecode, Error, Extern, Fiber, Lambda, Locals,
    NativeFnOp, Pattern, Result, Val,
};
use std::sync::{Arc, Mutex};
use tracing::{debug, warn};

/// A Lyric program process.
/// The [Proc] drives execution of [lyric::Fiber] to completion via
/// [lyric::Fiber]'s coroutine interface
#[derive(Debug)]
pub struct Proc<T: Extern, L: Locals> {
    fiber: Fiber<T, L>,
}

impl<T: Extern, L: Locals> Proc<T, L> {
    /// Run the program
    async fn run(&mut self) -> Result<Val<T, L>> {
        let mut fiber_result = self.fiber.start()?;
        while !self.fiber.is_done() {
            // TODO: Needs signal to run deferred async work from NativeFnAsync
            let async_result = async { Val::Nil }.await;
            fiber_result = self.fiber.resume(async_result)?;
        }
        Ok(fiber_result)
    }
}
