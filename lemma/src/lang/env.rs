use crate::lang::core;
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
        func: core::lang_list,
    });
    env.bind_special_form(SpecialForm {
        symbol: SymbolId::from("len"),
        func: core::lang_length,
    });
    env.bind_special_form(SpecialForm {
        symbol: SymbolId::from("get"),
        func: core::lang_get,
    });
    env.bind_special_form(SpecialForm {
        symbol: SymbolId::from("map"),
        func: core::lang_map,
    });
    env.bind_special_form(SpecialForm {
        symbol: SymbolId::from("push"),
        func: core::lang_push,
    });
    env.bind_special_form(SpecialForm {
        symbol: SymbolId::from("pushd"),
        func: core::lang_pushd,
    });
    env
}
