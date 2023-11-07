use crate::{builtin, Error, Extern, Lambda, Locals, NativeFn, SymbolId, Val};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

/// An environment of bindings
#[derive(Debug)]
pub struct Env<T: Extern, L: Locals> {
    bindings: HashMap<SymbolId, Val<T, L>>,
    parent: Option<EnvRef<T, L>>,
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
        };
        e.bind_native(SymbolId::from("contains"), builtin::contains_fn())
            .bind_native(SymbolId::from("eq?"), builtin::eq_fn())
            .bind_native(SymbolId::from("+"), builtin::plus_fn())
            .bind_native(SymbolId::from("ref"), builtin::ref_fn())
            .bind_native(SymbolId::from("list"), builtin::list_fn())
            .bind_native(SymbolId::from("push"), builtin::push_fn())
            .bind_native(SymbolId::from("get"), builtin::get_fn())
            .bind_native(SymbolId::from("map"), builtin::map_fn())
            .bind_native(SymbolId::from("not"), builtin::not_fn())
            .bind_native(SymbolId::from("ok?"), builtin::ok_fn())
            .bind_native(SymbolId::from("err?"), builtin::err_fn());

        e
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

    /// Extend an existing environment with given env as parent
    pub fn extend(parent: &Arc<Mutex<Env<T, L>>>) -> Self {
        Self {
            bindings: HashMap::new(),
            parent: Some(Arc::clone(parent)),
        }
    }

    /// Convenience to bind native functions
    pub fn bind_native(&mut self, symbol: SymbolId, nativefn: NativeFn<T, L>) -> &mut Self {
        self.define(symbol, Val::NativeFn(nativefn));
        self
    }

    /// Convenience to bind lambdas
    pub fn bind_lambda(&mut self, symbol: SymbolId, lambda: Lambda<T, L>) -> &mut Self {
        self.define(symbol, Val::Lambda(lambda));
        self
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
        }
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
