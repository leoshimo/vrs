//! Expressions in Lemma

/// Expressions of valid syntax tree in Lemma
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Int(i32),
    String(String),
    Symbol(String),
    List(Vec<Expr>),
}
