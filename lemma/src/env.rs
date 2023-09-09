/// The Lemma environment that has all bindings
use crate::{Expr, Result};
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Env {
    bindings: HashMap<String, Binding>,
}

#[derive(Debug)]
pub enum Binding {
    /// Normal form bindings
    Normal(Expr),
    /// Special form bindings
    Special(fn(&[Expr]) -> Result<Expr>),
}

impl Env {
    /// Resolve a given symbol to a binding, if any
    pub fn resolve(&self, symbol: &str) -> Option<&Binding> {
        self.bindings.get(symbol)
    }

    /// Bind a given symbol to given expression
    pub fn bind(&mut self, symbol: &str, expr: Expr) {
        self.bindings
            .insert(symbol.to_string(), Binding::Normal(expr));
    }
}
