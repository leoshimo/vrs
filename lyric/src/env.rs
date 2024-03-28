use crate::{
    builtin, Error, Extern, KeywordId, Lambda, Locals, NativeAsyncFn, NativeFn, SymbolId, Val,
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

/// An environment of bindings
#[derive(Debug)]
pub struct Env<T: Extern, L: Locals> {
    bindings: HashMap<SymbolId, Val<T, L>>,
    parent: Option<EnvRef<T, L>>,
    completions: CompletionConfig,
    pub(crate) macros: Option<crate::macros::MacroEnv<T, L>>,
}

pub type EntityCompletions = HashMap<KeywordId, Vec<SymbolId>>;

#[derive(Debug, Clone, Default)]
struct CompletionConfig {
    local: EntityCompletions,
    imported: HashMap<KeywordId, EntityCompletions>,
}

// TODO: EnvRef as NewType? For ergonomic clone
/// Reference to an environment
pub type EnvRef<T, L> = Arc<Mutex<Env<T, L>>>;

impl<T: Extern, L: Locals> Env<T, L> {
    /// Create standard base env
    pub fn standard() -> Self {
        let mut e = Env {
            bindings: HashMap::default(),
            parent: None,
            completions: CompletionConfig::default(),
            macros: Some(crate::macros::MacroEnv::default()),
        };
        e.bind_native(SymbolId::from("contains?"), builtin::contains_fn())
            .bind_native(SymbolId::from("eq?"), builtin::eq_fn())
            .bind_native(SymbolId::from("+"), builtin::plus_fn())
            .bind_native(SymbolId::from("-"), builtin::minus_fn())
            .bind_native(SymbolId::from("ref"), builtin::ref_fn())
            .bind_native(SymbolId::from("list"), builtin::list_fn())
            .bind_native(SymbolId::from("list?"), builtin::types::list_predicate_fn())
            .bind_native(SymbolId::from("error"), builtin::types::error_fn())
            .bind_native(SymbolId::from("push"), builtin::push_fn())
            .bind_native(SymbolId::from("get"), builtin::get_fn())
            .bind_native(SymbolId::from("map"), builtin::map_fn())
            .bind_native(SymbolId::from("apply"), builtin::list::apply_fn())
            .bind_native(SymbolId::from("len"), builtin::len_fn())
            .bind_lambda(SymbolId::from("filter"), builtin::filter_fn())
            .bind_native(SymbolId::from("not?"), builtin::not_fn())
            .bind_native(SymbolId::from("ok?"), builtin::ok_fn())
            .bind_native(SymbolId::from("empty?"), builtin::empty_fn())
            .bind_native(SymbolId::from("keyword?"), builtin::is_keyword_fn())
            .bind_native(SymbolId::from("err?"), builtin::err_fn())
            .bind_native(SymbolId::from("str"), builtin::str_fn())
            .bind_native(SymbolId::from("join"), builtin::join_fn())
            .bind_native(SymbolId::from("split"), builtin::split_fn())
            .bind_native(SymbolId::from("format"), builtin::format_fn())
            .bind_native(SymbolId::from("display"), builtin::display_fn())
            .bind_native(SymbolId::from("pretty"), builtin::pretty_fn())
            .bind_native(SymbolId::from("dbg"), builtin::dbg_fn())
            .bind_native(
                SymbolId::from("eval_source"),
                crate::source::eval_source_fn(),
            )
            .bind_native(SymbolId::from("read"), builtin::read_fn())
            .bind_native(SymbolId::from("help"), builtin::help_fn())
            .bind_native(SymbolId::from("meta"), builtin::metadata::meta_fn())
            .bind_native(
                SymbolId::from("set_entity_completions"),
                builtin::env::set_entity_completions_fn(),
            )
            .bind_native(
                SymbolId::from("get_entity_completions"),
                builtin::env::get_entity_completions_fn(),
            )
            .bind_native(SymbolId::from("ls_env"), builtin::ls_env_fn());

        crate::macros::bind_builtins(&mut e);

        e
    }

    /// Extend an existing environment with given env as parent
    pub fn extend(parent: &Arc<Mutex<Env<T, L>>>) -> Self {
        Self {
            bindings: HashMap::new(),
            parent: Some(Arc::clone(parent)),
            completions: CompletionConfig::default(),
            macros: None,
        }
    }

    /// Fork this environment in to a *deep copy*
    pub fn fork(&self) -> Self {
        let parent = self
            .parent
            .as_ref()
            .map(|parent| Arc::new(Mutex::new(parent.as_ref().lock().unwrap().clone())));
        Self {
            bindings: self.bindings.clone(),
            parent,
            completions: self.completions.clone(),
            macros: self.macros.clone(),
        }
    }

    /// Define a new symbol with given value in current environment
    pub fn define(&mut self, symbol: SymbolId, value: Val<T, L>) {
        self.bindings.insert(symbol.clone(), value);
    }

    /// Get value for symbol
    pub fn get(&self, symbol: &SymbolId) -> Option<Val<T, L>> {
        match self.bindings.get(symbol) {
            Some(v) => Some(v.clone()),
            None => self
                .parent
                .as_ref()
                .and_then(|p| p.lock().unwrap().get(symbol).clone()),
        }
    }

    /// Set value of symbol in lexical scope
    pub fn set(&mut self, symbol: &SymbolId, value: Val<T, L>) -> Result<(), Error> {
        if let Some(b) = self.bindings.get_mut(symbol) {
            *b = value;
            return Ok(());
        }

        if let Some(ref p) = self.parent {
            p.lock().unwrap().set(symbol, value)?;
            return Ok(());
        }

        Err(Error::UndefinedSymbol(symbol.clone()))
    }

    /// Convenience to bind native functions
    pub fn bind_native(&mut self, symbol: SymbolId, nativefn: NativeFn<T, L>) -> &mut Self {
        self.define(symbol, Val::NativeFn(nativefn));
        self
    }

    /// Convenience to bind native functions
    pub fn bind_native_async(
        &mut self,
        symbol: SymbolId,
        nativefn: NativeAsyncFn<T, L>,
    ) -> &mut Self {
        self.define(symbol, Val::NativeAsyncFn(nativefn));
        self
    }

    /// Convenience to bind lambdas
    pub fn bind_lambda(&mut self, symbol: SymbolId, lambda: Lambda<T, L>) -> &mut Self {
        self.define(symbol, Val::Lambda(lambda));
        self
    }

    /// Iterate over all symbols and bindings
    pub fn iter(&self) -> EnvIter<'_, T, L> {
        EnvIter(self.bindings.iter())
    }

    /// Visible names, including parents, with ordinary lexical shadowing.
    pub fn symbols(&self) -> Vec<SymbolId> {
        let mut symbols = self
            .parent
            .as_ref()
            .map(|p| p.lock().unwrap().symbols())
            .unwrap_or_default();
        symbols.extend(self.bindings.keys().cloned());
        symbols.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        symbols.dedup();
        symbols
    }

    pub(crate) fn macro_definition(
        &self,
        name: &str,
    ) -> crate::Result<crate::macros::Definition<T, L>> {
        match (&self.macros, &self.parent) {
            (Some(macros), _) => macros.get(name),
            (None, Some(parent)) => parent.lock().unwrap().macro_definition(name),
            (None, None) => crate::macros::MacroEnv::default().get(name),
        }
    }

    /// Snapshot the macro namespace when creating a process or installing a library.
    pub fn macro_env(&self) -> crate::macros::MacroEnv<T, L> {
        self.macros
            .clone()
            .or_else(|| self.parent.as_ref().map(|p| p.lock().unwrap().macro_env()))
            .unwrap_or_default()
    }

    pub fn set_macro_env(&mut self, macros: crate::macros::MacroEnv<T, L>) {
        self.macros = Some(macros);
    }

    pub fn set_entity_completions(&mut self, ty: KeywordId, providers: Option<Vec<SymbolId>>) {
        match providers {
            Some(providers) => {
                self.completions.local.insert(ty, providers);
            }
            None => {
                self.completions.local.remove(&ty);
            }
        }
    }

    pub fn import_entity_completions(&mut self, service: KeywordId, providers: EntityCompletions) {
        self.completions.imported.insert(service, providers);
    }

    fn completion_config(&self) -> CompletionConfig {
        let mut config = self
            .parent
            .as_ref()
            .map(|p| p.lock().unwrap().completion_config())
            .unwrap_or_default();
        config.local.extend(self.completions.local.clone());
        config.imported.extend(self.completions.imported.clone());
        config
    }

    pub fn entity_completions(&self) -> EntityCompletions {
        let config = self.completion_config();
        let mut result = EntityCompletions::new();
        let mut defaults = config.imported.into_iter().collect::<Vec<_>>();
        defaults.sort_by(|(a, _), (b, _)| a.as_str().cmp(b.as_str()));
        for (_, types) in defaults {
            for (ty, providers) in types {
                let values = result.entry(ty).or_default();
                for provider in providers {
                    if !values.contains(&provider) {
                        values.push(provider);
                    }
                }
            }
        }
        result.extend(config.local);
        result
    }
}

impl<T: Extern, L: Locals> std::clone::Clone for Env<T, L> {
    // Clone by value, not by ref via Arc::clone
    fn clone(&self) -> Self {
        let parent = self
            .parent
            .as_ref()
            .map(|parent| Arc::new(Mutex::new(parent.as_ref().lock().unwrap().clone())));
        Self {
            bindings: self.bindings.clone(),
            parent,
            completions: self.completions.clone(),
            macros: self.macros.clone(),
        }
    }
}

pub struct EnvIter<'a, T: Extern, L: Locals>(
    std::collections::hash_map::Iter<'a, SymbolId, Val<T, L>>,
);

impl<'a, T: Extern, L: Locals> Iterator for EnvIter<'a, T, L> {
    type Item = (&'a SymbolId, &'a Val<T, L>);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use void::Void;

    type Val = super::Val<Void, Void>;
    type Env = super::Env<Void, Void>;

    #[test]
    fn get() {
        let mut env = Env::standard();
        env.define(SymbolId::from("x"), Val::Int(0));
        assert_eq!(env.get(&SymbolId::from("x")), Some(Val::Int(0)));
    }

    #[test]
    fn get_undefined() {
        let env = Env::standard();
        assert_eq!(env.get(&SymbolId::from("x")), None)
    }

    #[test]
    fn set_defined() {
        let mut env = Env::standard();
        let sym = SymbolId::from("x");
        env.define(sym.clone(), Val::Int(0));
        assert_eq!(env.set(&sym, Val::string("one")), Ok(()));
        assert_eq!(
            env.get(&sym),
            Some(Val::string("one")),
            "Should get new value"
        );
    }

    #[test]
    fn set_undefined() {
        let sym = SymbolId::from("x");
        let mut env = Env::standard();
        assert_eq!(
            env.set(&sym, Val::Int(1)),
            Err(Error::UndefinedSymbol(SymbolId::from("x")))
        );
    }

    #[test]
    fn get_parent() {
        let sym = SymbolId::from("x");
        let parent = Arc::new(Mutex::new(Env::standard()));
        parent
            .lock()
            .unwrap()
            .define(sym.clone(), Val::keyword("parent"));

        let child = Env::extend(&parent);
        assert_eq!(
            child.get(&sym),
            Some(Val::keyword("parent")),
            "should get parent scope's value"
        );
    }

    #[test]
    fn set_parent() {
        let parent = Arc::new(Mutex::new(Env::standard()));
        let sym = SymbolId::from("x");
        parent
            .lock()
            .unwrap()
            .define(sym.clone(), Val::string("parent"));

        let mut child = Env::extend(&parent);
        assert_eq!(
            child.set(&sym, Val::string("updated")),
            Ok(()),
            "set from child scope should succeed"
        );

        assert_eq!(child.get(&sym), Some(Val::string("updated")),);
        assert_eq!(
            parent.lock().unwrap().get(&sym),
            Some(Val::string("updated")),
            "get should retrieve updated value"
        );
    }

    // TODO: Test Clone Isolation
}
