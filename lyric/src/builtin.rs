//! Builtin func
use crate::{Extern, NativeFn, NativeFnVal, SymbolId, Val};

/// Native binding for `+`
pub fn plus_fn<T: Extern>() -> NativeFn<T> {
    // TODO: Write tests for +
    // TODO: Support N operands
    NativeFn {
        symbol: SymbolId::from("+"),
        func: |_, x| match x {
            [Val::Int(a), Val::Int(b)] => Ok(NativeFnVal::Return(Val::Int(a + b))),
            _ => panic!("only supports ints"),
        },
    }
}
