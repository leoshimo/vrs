//! Builtins for unique reference type
//! This type is used to fill similar function as `make_ref()` in Erlang
use crate::{Extern, NativeFn, NativeFnVal, SymbolId, Val};
use nanoid::nanoid;

/// Unique reference type
#[derive(Debug, Clone, PartialEq)]
pub struct Ref(pub(crate) String);

/// Binding to create a new unique reference
pub fn ref_fn<T: Extern>() -> NativeFn<T> {
    NativeFn {
        symbol: SymbolId::from("ref"),
        func: |_, _| {
            let r = Ref(nanoid!());
            Ok(NativeFnVal::Return(Val::Ref(r)))
        },
    }
}
