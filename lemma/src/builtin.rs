//! Builtin func
use crate::{NativeFn, NativeFnVal, SymbolId, Val};

/// Native binding for "+" operator
pub fn plus_fn() -> NativeFn {
    // TODO: Write tests for builtins
    // TODO: Support N operands
    NativeFn {
        symbol: SymbolId::from("+"),
        func: |_, x| match x {
            [Val::Int(a), Val::Int(b)] => Ok(NativeFnVal::Return(Val::Int(a + b))),
            _ => panic!("only supports ints"),
        },
    }
}
