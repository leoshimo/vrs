/// The Lemma environment that has all bindings
use crate::Value;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Env {
    bindings: HashMap<String, Value>,
}

impl Env {
    /// Resolve a given symbol to a binding, if any
    pub fn resolve(&self, symbol: &str) -> Option<&Value> {
        self.bindings.get(symbol)
    }

    /// Bind a given symbol to given form
    pub fn bind(&mut self, symbol: &str, value: Value) {
        self.bindings.insert(symbol.to_string(), value);
    }
}
