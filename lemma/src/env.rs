/// The Lemma environment that has all bindings
use crate::{SymbolId, Value};
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Env {
    bindings: HashMap<SymbolId, Value>,
}

impl Env {
    /// Resolve a given symbol to a binding, if any
    pub fn resolve(&self, symbol: &SymbolId) -> Option<&Value> {
        self.bindings.get(symbol)
    }

    /// Bind a given symbol to given form
    pub fn bind(&mut self, symbol: &SymbolId, value: Value) {
        self.bindings.insert(symbol.clone(), value);
    }
}
