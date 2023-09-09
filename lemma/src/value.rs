//! Values in Lemma
//! A value is the result of evaluating an [Form](crate::Form)

use crate::{form, Env, Result, SymbolId};

/// A value from evaluating a [Form](crate::Form).
///
/// # Difference between [Form](crate::Form) and [Value]
/// All forms can be values, but not all values are forms due to function
/// bindings, special forms, and macro expansions.
/// [Value] is not serializable, but [Form](crate::Form) is.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// Integer value
    Int(i32),
    /// String value
    String(String),
    /// Form value
    Form(form::Form),
    /// Callable function value
    Func(Lambda),
}

/// Parameters that function accepts
pub type Params = Vec<SymbolId>;

/// A function that accepts argument values and env to compute another value
#[derive(Debug, Clone, PartialEq)]
pub struct Lambda {
    pub name: String,
    pub params: Params,
    pub func: fn(&Env) -> Result<Value>,
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Self::String(value.to_string())
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<form::Form> for Value {
    fn from(value: form::Form) -> Self {
        match value {
            form::Form::Int(i) => Self::Int(i),
            form::Form::String(s) => Self::String(s),
            _ => Self::Form(value),
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Int(i) => write!(f, "{}", i),
            Value::String(s) => write!(f, "{}", s),
            Value::Form(form) => write!(f, "{}", form),
            Value::Func(l) => write!(f, "<fn {}>", l.name),
        }
    }
}
