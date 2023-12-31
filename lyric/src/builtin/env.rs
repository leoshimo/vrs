//! Environment related bindings
use crate::{Error, Extern, Locals, NativeFn, NativeFnOp, Val};

/// Binding for ls-env builtin for dumping environment variables in current scope
pub fn ls_env_fn<T: Extern, L: Locals>() -> NativeFn<T, L> {
    NativeFn {
        func: |f, args| {
            if !args.is_empty() {
                return Err(Error::UnexpectedArguments(
                    "ls-env is not expected to have arguments".to_string(),
                ));
            }
            let mut res = vec![];
            {
                let env = f.cur_env().lock().unwrap();
                for (sym, val) in env.iter() {
                    res.push(Val::List(vec![Val::Symbol(sym.clone()), val.clone()]));
                }
            }
            Ok(NativeFnOp::Return(Val::List(res)))
        },
    }
}
