//! Builtins for unique reference type
//! This type is used to fill similar function as `make_ref()` in Erlang
use crate::{Extern, Locals, NativeFn, NativeFnOp, Val};
use nanoid::nanoid;
use serde::{Deserialize, Serialize};

/// Unique reference type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ref(pub(crate) String);

impl Ref {
    /// Create a reference that is unique within the running system.
    pub fn unique() -> Self {
        Self(nanoid!())
    }
}

/// Binding to create a new unique reference
pub fn ref_fn<T: Extern, L: Locals>() -> NativeFn<T, L> {
    NativeFn {
        doc: "(ref) - Creates a new unique reference in runtime".to_string(),
        func: |_, _| {
            let r = Ref::unique();
            Ok(NativeFnOp::Return(Val::Ref(r)))
        },
    }
}
