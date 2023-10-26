//! Pattern Matching
use crate::{Extern, Locals, Val};

/// Pattern matching predicate
pub struct Pattern<T: Extern, L: Locals> {
    inner: Val<T, L>,
}

/// Result of pattern match
pub struct Match {}

impl<T, L> Pattern<T, L>
where
    T: Extern,
    L: Locals,
{
    pub fn from_val(inner: Val<T, L>) -> Self {
        Self { inner }
    }

    /// Check if pattern matches given value
    pub fn matches(&self, val: &Val<T, L>) -> bool {
        Self::val_matches(&self.inner, val)
    }

    fn val_matches(pat: &Val<T, L>, val: &Val<T, L>) -> bool {
        use Val::*;

        match pat {
            Symbol(_) => true,
            List(pat) => match val {
                List(val) if pat.len() == val.len() => pat
                    .iter()
                    .zip(val.iter())
                    .all(|(lhs, rhs)| Self::val_matches(lhs, rhs)),
                _ => false,
            },
            Nil | Bool(_) | Int(_) | String(_) | Keyword(_) | Lambda(_) | NativeFn(_)
            | Bytecode(_) | Error(_) | Ref(_) | Extern(_) => pat == val,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{parse, Ref};
    use void::Void;

    type Val = crate::Val<Void, ()>;
    type Pattern = crate::Pattern<Void, ()>;

    #[test]
    fn values_matches_itself() {
        {
            assert!(Pattern::from_val(v("nil")).matches(&v("nil")));
            assert!(!Pattern::from_val(v("nil")).matches(&v("0")));
            assert!(!Pattern::from_val(v("nil")).matches(&v("\"hello\"")));
            assert!(!Pattern::from_val(v("nil")).matches(&v("true")));
        }
        {
            assert!(Pattern::from_val(v("true")).matches(&v("true")));
            assert!(Pattern::from_val(v("false")).matches(&v("false")));
            assert!(!Pattern::from_val(v("true")).matches(&v("false")));
            assert!(!Pattern::from_val(v("false")).matches(&v("true")));
        }
        {
            assert!(Pattern::from_val(v("0")).matches(&v("0")));
            assert!(Pattern::from_val(v("9999")).matches(&v("9999")));
            assert!(!Pattern::from_val(v("10")).matches(&v("1")));
            assert!(!Pattern::from_val(v("99")).matches(&v("0")));
            assert!(!Pattern::from_val(v("9")).matches(&v("99")));
        }
        {
            assert!(Pattern::from_val(v("\"hello\"")).matches(&v("\"hello\"")));
            assert!(Pattern::from_val(v("\"hello world\"")).matches(&v("\"hello world\"")));
            assert!(!Pattern::from_val(v("\"hello\"")).matches(&v("\"goodbye\"")));
            assert!(!Pattern::from_val(v("\"hello world\"")).matches(&v("\"hello\"")));
            assert!(!Pattern::from_val(v("\"hello world\"")).matches(&v("\"world\"")));
        }
        {
            assert!(Pattern::from_val(v(":hello")).matches(&v(":hello")));
            assert!(Pattern::from_val(v(":hello_world")).matches(&v(":hello_world")));
            assert!(!Pattern::from_val(v(":hello")).matches(&v(":goodbye")));
            assert!(!Pattern::from_val(v(":hello_world")).matches(&v(":hello")));
            assert!(!Pattern::from_val(v(":hello_world")).matches(&v(":world")));
        }
        {
            let ref1 = Val::Ref(Ref("abc".to_string()));
            let ref2 = Val::Ref(Ref("ABC".to_string()));
            let ref3 = Val::Ref(Ref("abcde".to_string()));

            assert!(Pattern::from_val(ref1.clone()).matches(&ref1));
            assert!(!Pattern::from_val(ref1.clone()).matches(&ref2));
            assert!(!Pattern::from_val(ref1.clone()).matches(&ref3));
            assert!(!Pattern::from_val(ref3).matches(&ref2));
        }
    }

    #[test]
    fn symbols_matches_all() {
        let pat = Pattern::from_val(v("a"));

        assert!(pat.matches(&v("hello")));
        assert!(pat.matches(&v("5")));
        assert!(pat.matches(&v("\"hello\"")));
        assert!(pat.matches(&v(":hello")));
        assert!(pat.matches(&v("'()")));
        assert!(pat.matches(&v("'(1 2 3)")));
    }

    #[test]
    fn list_empty() {
        let pat = Pattern::from_val(v("()"));
        assert!(pat.matches(&v("()")));
        assert!(!pat.matches(&v("(1)")));
        assert!(!pat.matches(&v("\"hello\"")));
    }

    #[test]
    fn list_nonempty() {
        let pat = Pattern::from_val(v("(a b c)"));
        assert!(pat.matches(&v("(1 2 3)")));
        assert!(pat.matches(&v("(:one :two \"three\")")));
        assert!(!pat.matches(&v("\"hello\"")));
        assert!(!pat.matches(&v("()")));
        assert!(!pat.matches(&v("(1 :two)")));
    }

    #[test]
    fn list_nested() {
        let pat = Pattern::from_val(v("(a b (c d))"));
        assert!(pat.matches(&v("(1 2 (3 4))")));
        assert!(pat.matches(&v("(:one :two (\"three\" 4))")));

        assert!(!pat.matches(&v("(1 2 3 4)")));
        assert!(!pat.matches(&v("(1 (2 3) 4)")));
        assert!(!pat.matches(&v("\"1234\"")));
        assert!(!pat.matches(&v("()")));
    }

    #[test]
    fn list_symbols_and_constants() {
        let pat = Pattern::from_val(v("(a 2 (:three d))"));
        assert!(pat.matches(&v("(:one 2 (:three 4))")));
        assert!(pat.matches(&v("(:one 2 (:three :four))")));

        assert!(!pat.matches(&v("(:one :two (\"three\" 4))")));
        assert!(!pat.matches(&v("(1 2 (3 4))")));
        assert!(!pat.matches(&v("()")));
    }

    fn v(expr: &str) -> Val {
        parse(expr).unwrap().into()
    }
}
