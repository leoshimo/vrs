use crate::{Extern, Locals, NativeFn, NativeFnOp, Val};

pub(crate) fn dbg_fn<T: Extern, L: Locals>() -> NativeFn<T, L> {
    NativeFn {
        func: |_, args| {
            println!("{:?}", &args);
            Ok(NativeFnOp::Return(Val::keyword("ok")))
        },
    }
}
