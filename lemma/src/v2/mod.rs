#![allow(dead_code)] // TODO: Remove me

mod codegen;
mod fiber;

pub(crate) use codegen::Inst;

pub use fiber::{start, Fiber};
