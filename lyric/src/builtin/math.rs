//! Math builtins
use crate::{Extern, Locals, NativeFn, NativeFnOp, Val};

/// Native binding for `+`
pub fn plus_fn<T: Extern, L: Locals>() -> NativeFn<T, L> {
    // TODO: Write tests for +
    // TODO: Support N operands
    NativeFn {
        func: |_, x| match x {
            [Val::Int(a), Val::Int(b)] => Ok(NativeFnOp::Return(Val::Int(a + b))),
            [Val::List(lhs), Val::List(rhs)] => {
                let mut items = lhs.to_vec();
                items.extend_from_slice(rhs);
                Ok(NativeFnOp::Return(Val::List(items)))
            }
            _ => panic!("only supports ints"),
        },
    }
}
