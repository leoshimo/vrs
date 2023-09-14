//! The virtual machine connecting components in the runtime

use std::thread;
use std::process::Command;

use lemma::{Env, Form, SymbolId, Value, SpecialForm, eval};

/// Handle to virtual machine environment
#[derive(Debug)]
pub struct Machine<'a> {
    /// Interpreter environment
    env: lemma::Env<'a>,
}

/// The commands dispatched to machine
pub type Message = lemma::Form;

/// Errors from machine
pub type Error = lemma::Error;

/// The results from machine
pub type Result = lemma::Result<lemma::Value>;

impl Machine<'_> {
    pub fn new() -> Self {
        let mut env = lemma::lang::std_env();
        add_open(&mut env);
        Self {
            env,
        }
    }

    /// Dispatch a command to be processed by machine
    pub fn dispatch(&mut self, cmd: &Message) -> Result {
        lemma::eval(cmd, &mut self.env)
    }

}

impl Default for Machine<'_> {
    fn default() -> Self {
        Self::new()
    }
}

fn add_open(env: &mut Env) {
    let sym = SymbolId::from("open");
    env.bind(
        &sym,
        Value::SpecialForm(SpecialForm {
            name: sym.to_string(),
            func: machine_open
        }),
    );
}

// TODO: Use tokio::process::Command
fn machine_open(arg_forms: &[Form], env: &mut Env) -> Result {
    let spec = match arg_forms {
        [f] => Ok(f),
        _ => Err(Error::UnexpectedArguments("open expects one argument only".to_string())),
    }?;

    let arg = match eval(spec, env)? {
        Value::Form(Form::String(s)) => Ok(s),
        _ => Err(Error::UnexpectedArguments("open expects one string argument".to_string())),
    }?;

    thread::spawn(|| {
        let _ = Command::new("open")
            .arg(arg)
            .spawn();
    });

    Ok(Value::from(Form::symbol("ok")))
}
