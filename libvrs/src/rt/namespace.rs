#![allow(unused_variables)]
//! Namespace for a process, wraping Lisp environment
use crate::rt::Result;

/// A namespace for specific process
pub(crate) struct Namespace {}

impl Namespace {
    /// Create a new expression
    pub(crate) fn new() -> Self {
        Self {}
    }

    /// Evaluate given expression
    pub(crate) fn eval(&mut self, form: &lemma::Expr) -> Result<lemma::Expr> {
        Ok(lemma::Expr::Int(0)) // TODO: Fix me
    }
}
