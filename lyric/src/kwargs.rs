//! Keyword list utilities

use crate::{Extern, KeywordId, Locals, Val};

/// Get the value associated with `keyword` from `lst`, if any.
pub fn get<T: Extern, L: Locals>(lst: &[Val<T, L>], target: &KeywordId) -> Option<Val<T, L>> {
    lst.windows(2)
        .find(|w| matches!(&w[0], Val::Keyword(key) if key == target))
        .map(|w| w[1].clone())
}

/// Check for existance of flag option, if any
/// Supports both implicit and explicit values:
/// - (my_func :my_flag)
/// - (my_func :my_flag true) / (my_func :my_flag false)
///
/// If value after `:my_flag` is not a boolean, it is not considered a value for flag argument
pub fn flag<T: Extern, L: Locals>(lst: &[Val<T, L>], target: &KeywordId) -> Option<Val<T, L>> {
    let mut iter = lst
        .iter()
        .skip_while(|v| matches!(v, Val::Keyword(kwd) if kwd != target));

    if iter.next().is_some() {
        let val = match iter.next() {
            Some(Val::Bool(b)) => Val::Bool(*b), // explicit flag
            _ => Val::Bool(true),                // implicit true flag if missing or not boolean
        };
        Some(val)
    } else {
        None // no explicit arg
    }
}

// TODO: Tests for `kwargs::get`
// TODO: Tests for `kwargs::flag`
