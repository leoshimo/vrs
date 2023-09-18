//! The virtual machine connecting components in the runtime

use std::process::Command;
use std::thread;

use lemma::{eval, Env, Form, SpecialForm, SymbolId, Value};

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
        add_exec(&mut env);
        Self { env }
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

fn add_exec(env: &mut Env) {
    env.bind_special_form(SpecialForm {
        symbol: SymbolId::from("exec"),
        func: machine_exec,
    });
}

// TODO: Use tokio::process::Command
fn machine_exec(arg_forms: &[Form], env: &mut Env) -> Result {
    let args = arg_forms
        .iter()
        .map(|f| match eval(f, env)? {
            Value::Form(Form::String(s)) => Ok(s),
            _ => Err(Error::UnexpectedArguments(
                "exec can only be passed string values".to_string(),
            )),
        })
        .collect::<lemma::Result<Vec<String>>>()?;

    let (cmd, args) = match args.split_first() {
        Some((cmd, args)) => Ok((cmd.clone(), args.to_owned())),
        None => Err(Error::UnexpectedArguments(
            "No arguments provided".to_string(),
        )),
    }?;

    thread::spawn(move || {
        let _ = Command::new(cmd).args(args).spawn();
    });

    Ok(Value::from(Form::symbol("ok")))
}
