//! Builtin functions
pub mod cond;
pub mod list;
pub mod math;
pub mod refs;
pub mod types;

// TODO: Builtins to instruction?

pub(crate) use cond::contains_fn;
pub(crate) use cond::eq_fn;
pub(crate) use cond::not_fn;
pub(crate) use list::get_fn;
pub(crate) use list::list_fn;
pub(crate) use list::map_fn;
pub(crate) use list::push_fn;
pub(crate) use math::plus_fn;
pub(crate) use refs::ref_fn;
pub(crate) use types::err_fn;

pub use refs::Ref;
