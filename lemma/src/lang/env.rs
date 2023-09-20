use crate::lang::{core, list};
use crate::{Env, NativeFunc, SymbolId};

/// Returns the 'standard' environment of the language
pub fn std_env<'a>() -> Env<'a> {
    let mut env = Env::new();
    env.bind_native(NativeFunc {
        symbol: SymbolId::from("lambda"),
        func: core::lang_lambda,
    });
    env.bind_native(NativeFunc {
        symbol: SymbolId::from("quote"),
        func: core::lang_quote,
    });
    env.bind_native(NativeFunc {
        symbol: SymbolId::from("eval"),
        func: core::lang_eval,
    });
    env.bind_native(NativeFunc {
        symbol: SymbolId::from("def"),
        func: core::lang_def,
    });
    env.bind_native(NativeFunc {
        symbol: SymbolId::from("if"),
        func: core::lang_if,
    });
    env.bind_native(NativeFunc {
        symbol: SymbolId::from("list"),
        func: list::lang_list,
    });
    env.bind_native(NativeFunc {
        symbol: SymbolId::from("len"),
        func: list::lang_len,
    });
    env.bind_native(NativeFunc {
        symbol: SymbolId::from("get"),
        func: list::lang_get,
    });
    env.bind_native(NativeFunc {
        symbol: SymbolId::from("getn"),
        func: list::lang_getn,
    });
    env.bind_native(NativeFunc {
        symbol: SymbolId::from("map"),
        func: list::lang_map,
    });
    env.bind_native(NativeFunc {
        symbol: SymbolId::from("push"),
        func: list::lang_push,
    });
    env.bind_native(NativeFunc {
        symbol: SymbolId::from("pop"),
        func: list::lang_pop,
    });
    env
}
