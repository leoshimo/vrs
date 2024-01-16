//! Environment related bindings
use crate::{Error, Extern, Locals, NativeFn, NativeFnOp, Val};

/// Binding for ls_env builtin for dumping environment variables in current scope
pub fn ls_env_fn<T: Extern, L: Locals>() -> NativeFn<T, L> {
    NativeFn {
        doc: "(ls_env EXPR) - Returns list of symbols defined in environment".to_string(),
        func: |f, args| {
            if !args.is_empty() {
                return Err(Error::UnexpectedArguments(
                    "ls_env is not expected to have arguments".to_string(),
                ));
            }
            let mut res = vec![];
            {
                let env = f.cur_env().lock().unwrap();
                for (sym, _) in env.iter() {
                    res.push(Val::Symbol(sym.clone()));
                }
            }
            Ok(NativeFnOp::Return(Val::List(res)))
        },
    }
}
