/// The Lemma environment that has all bindings
use crate::{Form, Result};
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Env {
    bindings: HashMap<String, Binding>,
}

#[derive(Debug)]
pub enum Binding {
    /// Normal form bindings
    Normal(Form),
    /// Function operator bindings
    Function {
        name: String,
        func: fn(&[Form]) -> Result<Form>,
    },
    /// Special form bindings
    Special(fn(&[Form]) -> Result<Form>),
}

impl Env {
    /// Resolve a given symbol to a binding, if any
    pub fn resolve(&self, symbol: &str) -> Option<&Binding> {
        self.bindings.get(symbol)
    }

    /// Bind a given symbol to given form
    pub fn bind(&mut self, symbol: &str, form: Form) {
        self.bindings
            .insert(symbol.to_string(), Binding::Normal(form));
    }
}
