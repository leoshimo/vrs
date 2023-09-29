//! The namespace for processes
use super::v2::Result;

/// The namespace for process
#[derive(Debug)]
pub struct Namespace<'a> {
    /// Environment of interpreter
    env: lemma::Env<'a>,
}

impl Namespace<'_> {
    /// Create the namespace
    pub(crate) fn new() -> Self {
        Self {
            env: lemma::lang::std_env(),
        }
    }

    /// Evaluate expression in namespace
    pub(crate) fn eval(&mut self, form: &lemma::Form) -> Result<lemma::Form> {
        Ok(lemma::eval(form, &mut self.env)?)
    }
}
