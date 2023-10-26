//! Math builtins
use crate::{Extern, Locals, NativeFn, NativeFnOp, SymbolId, Val};

/// Native binding for `+`
pub fn plus_fn<T: Extern, L: Locals>() -> NativeFn<T, L> {
    // TODO: Write tests for +
    // TODO: Support N operands
    NativeFn {
        symbol: SymbolId::from("+"),
        func: |_, x| match x {
            [Val::Int(a), Val::Int(b)] => Ok(NativeFnOp::Return(Val::Int(a + b))),
            _ => panic!("only supports ints"),
        },
    }
}
