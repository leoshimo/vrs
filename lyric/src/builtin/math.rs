//! Math builtins
use crate::{Extern, Locals, NativeFn, NativeFnOp, Result, Val};

/// Native binding for `+`
pub fn plus_fn<T: Extern, L: Locals>() -> NativeFn<T, L> {
    NativeFn {
        doc: "(+ LHS RHS) - If LHS and RHS are integers, returns sum of LHS and RHS.\
              If LHS and RHS are lists, returns a new list containing elements of LHS followed by elements of RHS.".to_string(),
        func: |_, args| match args {
            [Val::Int(_), ..] => {
                Ok(NativeFnOp::Return(
                    plus_add_ints(args)?
                ))
            },
            [Val::List(_), ..] => {
                Ok(NativeFnOp::Return(
                    plus_concat_list(args)?
                ))
            }
            _ => panic!("only supports ints"),
        },
    }
}

/// Native binding for `+` for adding integers
fn plus_add_ints<T: Extern, L: Locals>(args: &[Val<T, L>]) -> Result<Val<T, L>> {
    let mut res = 0;
    for arg in args {
        let num = arg.as_int()?;
        res += num;
    }
    Ok(Val::Int(res))
}

/// Native binding for `+` for concatenating lists
fn plus_concat_list<T: Extern, L: Locals>(args: &[Val<T, L>]) -> Result<Val<T, L>> {
    let mut result = vec![];
    for arg in args {
        let lst = arg.as_list()?;
        result.extend(lst.iter().cloned());
    }
    Ok(Val::List(result))
}

// TODO: Write tests for +
