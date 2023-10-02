//! Namespace for a process, wraping Lisp environment
use crate::rt::Result;

/// A namespace for specific process
pub(crate) struct Namespace<'a> {
    env: lemma::Env<'a>,
}

impl Namespace<'_> {
    /// Create a new expression
    pub(crate) fn new() -> Self {
        Self {
            env: lemma::lang::std_env(),
        }
    }

    /// Evaluate given expression
    pub(crate) fn eval(&mut self, form: &lemma::Form) -> Result<lemma::Form> {
        Ok(lemma::eval(form, &mut self.env)?)
    }
}
