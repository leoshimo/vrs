//! The virtual machine connecting components in the runtime

/// Handle to virtual machine environment
#[derive(Debug)]
pub struct Machine<'a> {
    /// Interpreter environment
    env: lemma::Env<'a>,
}

/// The commands dispatched to machine
pub type Command = lemma::Form;

/// Errors from machine
pub type Error = lemma::Error;

/// The results from machine
pub type Result = lemma::Result<lemma::Value>;

impl Machine<'_> {
    pub fn new() -> Self {
        Self {
            env: lemma::lang::std_env(),
        }
    }

    /// Dispatch a command to be processed by machine
    pub fn dispatch(&mut self, cmd: &Command) -> Result {
        lemma::eval(cmd, &mut self.env)
    }
}

impl Default for Machine<'_> {
    fn default() -> Self {
        Self::new()
    }
}
