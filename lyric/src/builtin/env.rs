//! Environment related bindings
use crate::{Error, Extern, Locals, NativeFn, NativeFnOp, Val};

/// Binding for ls_env builtin for dumping environment variables in current scope
pub fn ls_env_fn<T: Extern, L: Locals>() -> NativeFn<T, L> {
    NativeFn {
        metadata: vec![],
        doc: "(ls_env EXPR) - Returns list of symbols defined in environment".to_string(),
        func: |f, args| {
            if !args.is_empty() {
                return Err(Error::UnexpectedArguments(
                    "ls_env is not expected to have arguments".to_string(),
                ));
            }
            let mut res = vec![];
            {
                let env = f.cur_env().lock().unwrap();
                for sym in env.symbols() {
                    res.push(Val::Symbol(sym));
                }
            }
            Ok(NativeFnOp::Return(Val::List(res)))
        },
    }
}

pub(crate) fn register_entity_source_fn<T: Extern, L: Locals>() -> NativeFn<T, L> {
    NativeFn {
        metadata: vec![],
        doc: "(register_entity_source TYPE SOURCE) - Set source names for an entity type using a symbol or list; nil restores imported defaults".into(),
        func: |fiber, args| {
            let [Val::Keyword(ty), providers] = args else {
                return Err(Error::UnexpectedArguments("register_entity_source expects a type keyword and source names".into()));
            };
            let providers = match providers {
                Val::Symbol(symbol) => Some(vec![symbol.clone()]),
                Val::List(symbols) => Some(symbols.iter().map(|symbol| symbol.as_symbol().cloned()).collect::<crate::Result<Vec<_>>>()?),
                Val::Nil => None,
                _ => return Err(Error::UnexpectedArguments("entity sources must be symbols".into())),
            };
            fiber.global_env().lock().unwrap().register_entity_source(ty.clone(), providers);
            Ok(NativeFnOp::Return(Val::keyword("ok")))
        },
    }
}

pub(crate) fn entity_sources_fn<T: Extern, L: Locals>() -> NativeFn<T, L> {
    NativeFn {
        metadata: vec![],
        doc: "(entity_sources TYPE) - Return registered source names without invoking them".into(),
        func: |fiber, args| {
            let [Val::Keyword(ty)] = args else {
                return Err(Error::UnexpectedArguments(
                    "entity_sources expects a type keyword".into(),
                ));
            };
            let providers = fiber
                .global_env()
                .lock()
                .unwrap()
                .entity_completions()
                .remove(ty)
                .unwrap_or_default();
            Ok(NativeFnOp::Return(Val::List(
                providers.into_iter().map(Val::Symbol).collect(),
            )))
        },
    }
}
