use crate::{Extern, Locals, NativeFn, NativeFnOp, Val};

pub(crate) fn str_fn<T: Extern, L: Locals>() -> NativeFn<T, L> {
    NativeFn {
        func: |_, args| {
            let mut result = String::new();
            for v in args {
                result += &v.as_string_coerce()?;
            }
            Ok(NativeFnOp::Return(Val::String(result)))
        },
    }
}

// TODO: Test cases for str:
// (str "a " "b " "c")
// (str "a" " " "b" " " "c") # => "a b c"
// (str 5) # => 5
