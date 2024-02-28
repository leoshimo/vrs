//! Builtin functions
pub mod cond;
pub mod docs;
pub mod env;
pub mod list;
pub mod log;
pub mod math;
pub mod refs;
pub mod string;
pub mod types;

// TODO: Builtins to instruction?

pub(crate) use cond::contains_fn;
pub(crate) use cond::eq_fn;
pub(crate) use cond::not_fn;
pub(crate) use docs::help_fn;
pub(crate) use env::ls_env_fn;
pub(crate) use list::filter_fn;
pub(crate) use list::get_fn;
pub(crate) use list::list_fn;
pub(crate) use list::map_fn;
pub(crate) use list::push_fn;
pub(crate) use log::dbg_fn;
pub(crate) use math::plus_fn;
pub(crate) use refs::ref_fn;
pub(crate) use string::display_fn;
pub(crate) use string::format_fn;
pub(crate) use string::join_fn;
pub(crate) use string::read_fn;
pub(crate) use string::str_fn;
pub(crate) use types::err_fn;
pub(crate) use types::ok_fn;

pub use refs::Ref;
