//! Values in Lemma
//! A value is the result of evaluating an [Form](crate::Form)

use crate::{form, Result};

/// A value from evaluating a [Form](crate::Form).
///
/// # Difference between [Form](crate::Form) and [Value]
/// All forms can be values, but not all values are forms due to function
/// bindings, special forms, and macro expansions.
/// [Value] is not serializable, but [Form](crate::Form) is.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// Self-evaluating integer
    Int(i32),
    /// Self-evaluating string
    String(String),
    /// Self-evaluating form
    Form(form::Form),
    /// Function form
    Function { name: String, lambda: Lambda },
}

/// An function that can be evaluated with values to compute another value
pub type Lambda = fn(&[Value]) -> Result<Value>;

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
            Value::Function { name, .. } => write!(f, "<fn {}>", name),
        }
    }
}
