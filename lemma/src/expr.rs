//! Expressions in Lemma

/// Expressions of valid syntax tree in Lemma
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Int(i32),
    String(String),
    Symbol(String),
    List(Vec<Expr>),
}

impl std::fmt::Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Int(i) => write!(f, "{}", i),
            Expr::String(s) => write!(f, "\"{}\"", s),
            Expr::Symbol(s) => write!(f, "{}", s),
            Expr::List(l) => write!(
                f,
                "({})",
                l.iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display() {
        assert_eq!(Expr::Int(5).to_string(), "5");
        assert_eq!(Expr::String(String::from("hello")).to_string(), "\"hello\"");
        assert_eq!(Expr::Symbol(String::from("hello")).to_string(), "hello");
        assert_eq!(
            Expr::List(vec![
                Expr::Symbol(String::from("my-func")),
                Expr::Int(5),
                Expr::String(String::from("string")),
            ])
            .to_string(),
            "(my-func 5 \"string\")"
        );
    }
}
