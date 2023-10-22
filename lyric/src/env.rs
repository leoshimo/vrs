use crate::{builtin, Error, Extern, NativeFn, SymbolId, Val};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

/// An environment of bindings
#[derive(Debug)]
pub struct Env<T: Extern> {
    bindings: HashMap<SymbolId, Val<T>>,
    parent: Option<Arc<Mutex<Env<T>>>>,
}

impl<T: Extern> Env<T> {
    /// Create standard base env
    pub fn standard() -> Self {
        let mut e = Env {
            bindings: HashMap::default(),
            parent: None,
        };
        e.bind(builtin::plus_fn());
        e.bind(builtin::peval_fn());
        e
    }

    /// Define a new symbol with given value in current environment
    pub fn define(&mut self, symbol: &SymbolId, value: Val<T>) {
        self.bindings.insert(symbol.clone(), value);
    }

    /// Get value for symbol
    pub fn get(&self, symbol: &SymbolId) -> Option<Val<T>> {
        match self.bindings.get(symbol) {
            Some(v) => Some(v.clone()),
            None => self
                .parent
                .as_ref()
                .and_then(|p| p.lock().unwrap().get(symbol).clone()),
        }
    }

    /// Set value of symbol in lexical scope
    pub fn set(&mut self, symbol: &SymbolId, value: Val<T>) -> Result<(), Error> {
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
    pub fn extend(parent: &Arc<Mutex<Env<T>>>) -> Self {
        Self {
            bindings: HashMap::new(),
            parent: Some(Arc::clone(parent)),
        }
    }

    /// Convenience to bind native functions
    pub fn bind(&mut self, nativefn: NativeFn<T>) -> &mut Self {
        self.define(&nativefn.symbol.clone(), Val::NativeFn(nativefn));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use void::Void;

    type Val = super::Val<Void>;
    type Env = super::Env<Void>;

    #[test]
    fn get() {
        let mut env = Env::standard();
        env.define(&SymbolId::from("x"), Val::Int(0));
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
        env.define(&sym, Val::Int(0));
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
        parent.lock().unwrap().define(&sym, Val::keyword("parent"));

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
        parent.lock().unwrap().define(&sym, Val::string("parent"));

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
}
