use crate::{
    builtin, Error, Extern, KeywordId, Lambda, Locals, NativeAsyncFn, NativeFn, SymbolId, Val,
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, Weak},
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
            .bind_native(SymbolId::from("first"), builtin::list::first_fn())
            .bind_native(SymbolId::from("last"), builtin::list::last_fn())
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
                SymbolId::from("register_entity_source"),
                builtin::env::register_entity_source_fn(),
            )
            .bind_native(
                SymbolId::from("entity_sources"),
                builtin::env::entity_sources_fn(),
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

    /// Clone the macro namespace, retaining its lexical captures.
    /// Use Fiber::fork when starting an isolated execution.
    pub fn macro_env(&self) -> crate::macros::MacroEnv<T, L> {
        self.macros
            .clone()
            .or_else(|| self.parent.as_ref().map(|p| p.lock().unwrap().macro_env()))
            .unwrap_or_default()
    }

    pub fn set_macro_env(&mut self, macros: crate::macros::MacroEnv<T, L>) {
        self.macros = Some(macros);
    }

    pub fn register_entity_source(&mut self, ty: KeywordId, providers: Option<Vec<SymbolId>>) {
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

/// Copies the roots of one fiber, preserving shared scopes and cycles.
pub(crate) struct EnvCopy<T: Extern, L: Locals> {
    environments: HashMap<usize, EnvRef<T, L>>,
    // Keep source addresses stable while copying additional roots.
    sources: Vec<EnvRef<T, L>>,
}

impl<T: Extern, L: Locals> Default for EnvCopy<T, L> {
    fn default() -> Self {
        Self {
            environments: HashMap::new(),
            sources: Vec::new(),
        }
    }
}

impl<T: Extern, L: Locals> EnvCopy<T, L> {
    pub(crate) fn into_owner(self) -> OwnedEnvironments<T, L> {
        let scopes = self
            .environments
            .values()
            .map(Arc::downgrade)
            .collect::<Vec<_>>();
        let prune_at = (scopes.len() * 2).max(256);
        OwnedEnvironments { scopes, prune_at }
    }

    pub(crate) fn env(&mut self, source: &EnvRef<T, L>) -> EnvRef<T, L> {
        let key = Arc::as_ptr(source) as usize;
        if let Some(target) = self.environments.get(&key) {
            return target.clone();
        }
        let target = Arc::new(Mutex::new(Env {
            bindings: HashMap::new(),
            parent: None,
            completions: CompletionConfig::default(),
            macros: None,
        }));
        // Register before traversing captures: recursive functions and macro
        // definitions commonly point back to this very environment.
        self.sources.push(source.clone());
        self.environments.insert(key, target.clone());
        let (bindings, parent, completions, macros) = {
            let source = source.lock().unwrap();
            (
                source.bindings.clone(),
                source.parent.clone(),
                source.completions.clone(),
                source.macros.clone(),
            )
        }; // Never hold a source lock while following an edge.
        let copy = Env {
            bindings: bindings
                .into_iter()
                .map(|(k, v)| (k, self.value(&v)))
                .collect(),
            parent: parent.as_ref().map(|p| self.env(p)),
            completions,
            macros: macros.as_ref().map(|m| m.isolated_copy(self)),
        };
        *target.lock().unwrap() = copy;
        target
    }

    fn value(&mut self, value: &Val<T, L>) -> Val<T, L> {
        match value {
            Val::List(values) => Val::List(values.iter().map(|v| self.value(v)).collect()),
            Val::Lambda(lambda) => Val::Lambda(self.lambda(lambda)),
            Val::Bytecode(code) => Val::Bytecode(self.bytecode(code)),
            Val::Nil
            | Val::Bool(_)
            | Val::Int(_)
            | Val::String(_)
            | Val::Symbol(_)
            | Val::Keyword(_)
            | Val::NativeFn(_)
            | Val::NativeAsyncFn(_)
            | Val::Error(_)
            | Val::Ref(_)
            | Val::Extern(_) => value.clone(),
        }
    }

    pub(crate) fn lambda(&mut self, lambda: &Lambda<T, L>) -> Lambda<T, L> {
        Lambda {
            metadata: lambda.metadata.clone(),
            doc: lambda.doc.clone(),
            params: lambda.params.clone(),
            code: self.bytecode(&lambda.code),
            parent: lambda.parent.as_ref().map(|p| self.env(p)),
        }
    }

    pub(crate) fn bytecode(&mut self, code: &crate::Bytecode<T, L>) -> crate::Bytecode<T, L> {
        use crate::Inst::*;
        code.iter()
            .map(|inst| match inst {
                Prepare(v) => Prepare(self.value(v)),
                DefineMacro(v) => DefineMacro(self.value(v)),
                PushConst(v) => PushConst(self.value(v)),
                DebugScope(code, site) => DebugScope(self.bytecode(code), site.clone()),
                Expand(_)
                | ValidateExpansion
                | EvalCallsite
                | GetSym(_)
                | DefSym(_)
                | DefBind
                | SetSym(_)
                | MakeFunc
                | CallFunc(_)
                | CallAt(_, _)
                | CallCallback(_)
                | FunctionSource(_)
                | ListPush
                | ListExtend
                | PopTop
                | JumpFwd(_)
                | JumpBck(_)
                | PopJumpFwdIfTrue(_)
                | YieldTop
                | Eval(_) => inst.clone(),
            })
            .collect()
    }

    pub(crate) fn macros(
        &mut self,
        macros: &crate::macros::MacroEnv<T, L>,
    ) -> crate::macros::MacroEnv<T, L> {
        macros.isolated_copy(self)
    }
}

/// Scopes belong to one forked fiber. Break closure cycles when it is dropped.
/// Weak entries allow ordinary call frames to be freed during execution.
#[derive(Debug)]
pub(crate) struct OwnedEnvironments<T: Extern, L: Locals> {
    scopes: Vec<Weak<Mutex<Env<T, L>>>>,
    prune_at: usize,
}

impl<T: Extern, L: Locals> OwnedEnvironments<T, L> {
    pub(crate) fn register(&mut self, env: &EnvRef<T, L>) {
        if self.scopes.len() >= self.prune_at {
            self.scopes.retain(|scope| scope.strong_count() > 0);
            self.prune_at = (self.scopes.len() * 2).max(256);
        }
        self.scopes.push(Arc::downgrade(env));
    }
}

impl<T: Extern, L: Locals> Drop for OwnedEnvironments<T, L> {
    fn drop(&mut self) {
        // Keep every surviving scope alive until all edges have been removed.
        let scopes = self
            .scopes
            .iter()
            .filter_map(Weak::upgrade)
            .collect::<Vec<_>>();
        for scope in &scopes {
            let contents = {
                let mut scope = scope.lock().unwrap_or_else(|e| e.into_inner());
                (
                    std::mem::take(&mut scope.bindings),
                    scope.parent.take(),
                    scope.macros.take(),
                )
            };
            drop(contents); // Drop captures outside the scope lock.
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

    #[test]
    fn scope_tracking_prunes_dead_call_frames() {
        let mut owner = EnvCopy::<Void, Void>::default().into_owner();
        for _ in 0..10_000 {
            let scope = Arc::new(Mutex::new(Env::standard()));
            owner.register(&scope);
        }
        assert!(owner.scopes.len() <= 256);
    }

    #[test]
    fn isolated_copy_preserves_cycles_and_distinct_scopes() {
        let root = Arc::new(Mutex::new(Env::standard()));
        let inner = Arc::new(Mutex::new(Env::extend(&root)));
        let lambda = Lambda {
            metadata: vec![],
            doc: None,
            params: vec![],
            code: vec![],
            parent: Some(inner.clone()),
        };
        root.lock()
            .unwrap()
            .define("nested".into(), Val::Lambda(lambda));
        root.lock().unwrap().define("count".into(), Val::Int(0));
        inner.lock().unwrap().define("local".into(), Val::Int(10));

        let mut copy = EnvCopy::default();
        let copied_root = copy.env(&root);
        let copied_inner = copy.env(&inner);
        assert!(!Arc::ptr_eq(&root, &copied_root));
        assert!(!Arc::ptr_eq(&inner, &copied_inner));
        assert!(!Arc::ptr_eq(&copied_root, &copied_inner));
        let Val::Lambda(nested) = copied_root.lock().unwrap().get(&"nested".into()).unwrap() else {
            panic!("expected closure");
        };
        assert!(Arc::ptr_eq(nested.parent.as_ref().unwrap(), &copied_inner));
        assert!(Arc::ptr_eq(
            copied_inner.lock().unwrap().parent.as_ref().unwrap(),
            &copied_root
        ));
        copied_inner
            .lock()
            .unwrap()
            .set(&"count".into(), Val::Int(42))
            .unwrap();
        assert_eq!(
            copied_root.lock().unwrap().get(&"count".into()),
            Some(Val::Int(42))
        );
        assert_eq!(root.lock().unwrap().get(&"count".into()), Some(Val::Int(0)));
    }
}
