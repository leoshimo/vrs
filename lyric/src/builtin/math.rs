//! Math builtins
use crate::{Extern, Locals, NativeFn, NativeFnOp, Val};

/// Native binding for `+`
pub fn plus_fn<T: Extern, L: Locals>() -> NativeFn<T, L> {
    // TODO: Write tests for +
    // TODO: Support N operands
    NativeFn {
        doc: "(+ LHS RHS) - If LHS and RHS are integers, returns sum of LHS and RHS.\
              If LHS and RHS are lists, returns a new list containing elements of LHS followed by elements of RHS.".to_string(),
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
