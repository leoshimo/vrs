use crate::lang::{core, list};
use crate::{Env, SpecialForm, SymbolId};

/// Returns the 'standard' environment of the language
pub fn std_env<'a>() -> Env<'a> {
    let mut env = Env::new();
    env.bind_special_form(SpecialForm {
        symbol: SymbolId::from("lambda"),
        func: core::lang_lambda,
    });
    env.bind_special_form(SpecialForm {
        symbol: SymbolId::from("quote"),
        func: core::lang_quote,
    });
    env.bind_special_form(SpecialForm {
        symbol: SymbolId::from("eval"),
        func: core::lang_eval,
    });
    env.bind_special_form(SpecialForm {
        symbol: SymbolId::from("def"),
        func: core::lang_def,
    });
    env.bind_special_form(SpecialForm {
        symbol: SymbolId::from("if"),
        func: core::lang_if,
    });
    env.bind_special_form(SpecialForm {
        symbol: SymbolId::from("list"),
        func: list::lang_list,
    });
    env.bind_special_form(SpecialForm {
        symbol: SymbolId::from("len"),
        func: list::lang_len,
    });
    env.bind_special_form(SpecialForm {
        symbol: SymbolId::from("get"),
        func: list::lang_get,
    });
    env.bind_special_form(SpecialForm {
        symbol: SymbolId::from("getn"),
        func: list::lang_getn,
    });
    env.bind_special_form(SpecialForm {
        symbol: SymbolId::from("map"),
        func: list::lang_map,
    });
    env.bind_special_form(SpecialForm {
        symbol: SymbolId::from("push"),
        func: list::lang_push,
    });
    env.bind_special_form(SpecialForm {
        symbol: SymbolId::from("pushd"),
        func: list::lang_pushd,
    });
    env
}
