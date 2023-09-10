/// The Lemma environment that has all bindings
use crate::{SymbolId, Value};
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Env<'a> {
    bindings: HashMap<SymbolId, Value>,
    parent: Option<&'a Env<'a>>,
}

impl Env<'_> {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
            parent: None,
        }
    }

    /// Resolve a given symbol ID to the value in this environment
    pub fn resolve(&self, symbol: &SymbolId) -> Option<&Value> {
        if let Some(value) = self.bindings.get(symbol) {
            Some(value)
        } else if let Some(value) = self.parent.and_then(|p| p.resolve(symbol)) {
            Some(value)
        } else {
            None
        }
    }

    /// Bind a given symbol to given form
    pub fn bind(&mut self, symbol: &SymbolId, value: Value) {
        self.bindings.insert(symbol.clone(), value);
    }

    /// Create a new environment existing existing one
    pub(crate) fn extend<'a>(env: &'a Env<'a>) -> Env<'a> {
        Env {
            bindings: HashMap::new(),
            parent: Some(env),
        }
    }
}
