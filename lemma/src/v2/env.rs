use crate::{Form, SymbolId};
use std::collections::HashMap;

use super::fiber::FiberError;

/// An environment of bindings
#[derive(Debug)]
pub struct Env {
    scopes: Vec<Bindings>,
}

type Bindings = HashMap<SymbolId, Form>;

impl Env {
    /// Create a new environment
    pub fn new() -> Env {
        Self {
            scopes: vec![Bindings::new()],
        }
    }

    /// Define a new symbol with given value in current environment
    pub fn define(&mut self, symbol: &SymbolId, value: Form) {
        self.scopes
            .last_mut()
            .expect("Defining variable without scope")
            .insert(symbol.clone(), value);
    }

    /// Set value of symbol in lexical scope
    pub fn set(&mut self, symbol: &SymbolId, value: Form) -> Result<(), FiberError> {
        for s in self.scopes.iter_mut().rev() {
            match s.get_mut(symbol) {
                Some(b) => {
                    *b = value;
                    return Ok(());
                }
                None => continue,
            }
        }

        Err(FiberError::UndefinedSymbol(symbol.clone()))
    }

    /// Get value for symbol
    pub fn get(&self, symbol: &SymbolId) -> Option<Form> {
        for s in self.scopes.iter().rev() {
            if let Some(v) = s.get(symbol) {
                return Some(v.clone());
            }
        }
        None
    }

    /// Push a new environment scope
    pub fn push_scope(&mut self) {
        self.scopes.push(Bindings::new());
    }

    /// Pop a scope from environment
    pub fn pop_scope(&mut self) {
        self.scopes.pop().expect("Environment contains no scope");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_undefined() {
        let env = Env::new();
        assert_eq!(env.get(&SymbolId::from("x")), None)
    }

    #[test]
    fn define_get() {
        let mut env = Env::new();
        env.define(&SymbolId::from("x"), Form::Int(0));
        assert_eq!(env.get(&SymbolId::from("x")), Some(Form::Int(0)));
    }

    #[test]
    fn set_defined() {
        let mut env = Env::new();
        let sym = SymbolId::from("x");
        env.define(&sym, Form::Int(0));
        assert_eq!(env.set(&sym, Form::string("one")), Ok(()));
        assert_eq!(
            env.get(&sym),
            Some(Form::string("one")),
            "Should get new value"
        );
    }

    #[test]
    fn set_undefined() {
        let mut env = Env::new();
        assert_eq!(
            env.set(&SymbolId::from("x"), Form::Int(1)),
            Err(FiberError::UndefinedSymbol(SymbolId::from("x")))
        );
    }

    #[test]
    fn get_local() {
        let mut env = Env::new();
        let sym = SymbolId::from("x");

        env.push_scope();
        env.define(&sym, Form::Int(0));
        assert_eq!(
            env.get(&sym),
            Some(Form::Int(0)),
            "local scope should have value"
        );
        env.pop_scope();

        assert_eq!(
            env.get(&sym),
            None,
            "root scope should not have value defined"
        );
    }

    #[test]
    fn get_child_shadowed() {
        let mut env = Env::new();
        let sym = SymbolId::from("x");

        env.define(&sym, Form::string("outer"));

        env.push_scope(); // begin middle

        env.define(&sym, Form::string("middle"));
        assert_eq!(env.get(&sym), Some(Form::string("middle")));

        env.push_scope(); // begin inner
        env.define(&sym, Form::string("inner"));
        assert_eq!(env.get(&sym), Some(Form::string("inner")));
        env.pop_scope(); // end inner

        assert_eq!(
            env.get(&sym),
            Some(Form::string("middle")),
            "value in middle scope should be intact"
        );

        env.pop_scope(); // end middle

        assert_eq!(
            env.get(&sym),
            Some(Form::string("outer")),
            "value in outer scope should be intact"
        );
    }

    #[test]
    fn get_parent() {
        let mut env = Env::new();
        let sym = SymbolId::from("x");

        env.define(&sym, Form::string("parent"));
        env.push_scope();

        assert_eq!(
            env.get(&sym),
            Some(Form::string("parent")),
            "should be parent scope's value"
        );

        env.pop_scope();
    }

    #[test]
    fn set_parent() {
        let mut env = Env::new();
        let sym = SymbolId::from("x");

        env.define(&sym, Form::string("parent"));

        env.push_scope();

        assert_eq!(
            env.set(&sym, Form::string("updated")),
            Ok(()),
            "set from child scope should succeed"
        );

        env.pop_scope();

        assert_eq!(
            env.get(&sym),
            Some(Form::string("updated")),
            "get should retrieve updated value"
        );
    }
}
