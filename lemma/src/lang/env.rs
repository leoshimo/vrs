use crate::Env;

/// Returns the 'standard' environment of the language
pub fn std_env<'a>() -> Env<'a> {
    Env::new()
}
