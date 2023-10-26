//! Builtin functions
pub mod list;
pub mod math;
pub mod refs;

pub(crate) use list::get_fn;
pub(crate) use list::list_fn;
pub(crate) use list::push_fn;
pub(crate) use math::plus_fn;
pub(crate) use refs::ref_fn;

pub use refs::Ref;
