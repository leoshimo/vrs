//! Bindings to Open Things

use crate::rt::program::Lambda;
use lyric::{compile, parse, SymbolId};

// TODO: Move these to init.ll global bindings

/// Binding for open_url
pub(crate) fn open_url_fn() -> Lambda {
    Lambda {
        doc: Some("(open_url URL) - Opens URL in browser".to_string()),
        params: vec![SymbolId::from("url")],
        code: compile(&parse(r#"(exec "open" "-a" "Safari" url)"#).unwrap().into()).unwrap(),
        parent: None,
    }
}

/// Binding for open_app
pub(crate) fn open_app_fn() -> Lambda {
    Lambda {
        doc: Some(
            "(open_app APP_NAME) - Opens the application APP_NAME on host machine".to_string(),
        ),
        params: vec![SymbolId::from("app")],
        code: compile(&parse(r#"(exec "open" "-a" app)"#).unwrap().into()).unwrap(),
        parent: None,
    }
}

/// Binding for open_file
pub(crate) fn open_file_fn() -> Lambda {
    Lambda {
        doc: Some("(open_file FILE_NAME) - Opens the file FILE_NAME on host machine".to_string()),
        params: vec![SymbolId::from("file")],
        code: compile(
            &parse(r#"(exec "open" (shell_expand file))"#)
                .unwrap()
                .into(),
        )
        .unwrap(),
        parent: None,
    }
}
