use crate::{SymbolId, Val};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use super::fiber::FiberError;

/// An environment of bindings
#[derive(Debug, Default)]
pub struct Env {
    bindings: HashMap<SymbolId, Val>,
    parent: Option<Rc<RefCell<Env>>>,
}

impl Env {
    /// Define a new symbol with given value in current environment
    pub fn define(&mut self, symbol: &SymbolId, value: Val) {
        self.bindings.insert(symbol.clone(), value);
    }

    /// Get value for symbol
    pub fn get(&self, symbol: &SymbolId) -> Option<Val> {
        match self.bindings.get(symbol) {
            Some(v) => Some(v.clone()),
            None => self
                .parent
                .as_ref()
                .and_then(|p| p.borrow().get(symbol).clone()),
        }
    }

    /// Set value of symbol in lexical scope
    pub fn set(&mut self, symbol: &SymbolId, value: Val) -> Result<(), FiberError> {
        if let Some(b) = self.bindings.get_mut(symbol) {
            *b = value;
            return Ok(());
        }

        if let Some(ref p) = self.parent {
            p.borrow_mut().set(symbol, value)?;
            return Ok(());
        }

        Err(FiberError::UndefinedSymbol(symbol.clone()))
    }

    /// Extend an existing environment with given env as parent
    pub fn extend(parent: &Rc<RefCell<Env>>) -> Self {
        Self {
            bindings: HashMap::new(),
            parent: Some(Rc::clone(parent)),
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn get_undefined() {
//         let env = Env::default();
//         assert_eq!(env.get(&SymbolId::from("x")), None)
//     }

//     #[test]
//     fn define_get() {
//         let mut env = Env::default();
//         env.define(&SymbolId::from("x"), Form::Int(0));
//         assert_eq!(env.get(&SymbolId::from("x")), Some(Form::Int(0)));
//     }

//     #[test]
//     fn set_defined() {
//         let mut env = Env::default();
//         let sym = SymbolId::from("x");
//         env.define(&sym, Form::Int(0));
//         assert_eq!(env.set(&sym, Form::string("one")), Ok(()));
//         assert_eq!(
//             env.get(&sym),
//             Some(Form::string("one")),
//             "Should get new value"
//         );
//     }

//     #[test]
//     fn set_undefined() {
//         let mut env = Env::default();
//         assert_eq!(
//             env.set(&SymbolId::from("x"), Form::Int(1)),
//             Err(FiberError::UndefinedSymbol(SymbolId::from("x")))
//         );

//         let child = Env::extend(&parent);
//         assert_eq!(
//             child.get(&sym),
//             Some(Form::string("parent")),
//             "should be parent scope's value"
//         );
//     }

//     #[test]
//     fn set_parent() {
//         let parent = Rc::new(RefCell::new(Env::default()));
//         let sym = SymbolId::from("x");
//         parent.borrow_mut().define(&sym, Form::string("parent"));

//         let mut child = Env::extend(&parent);
//         assert_eq!(
//             child.set(&sym, Form::string("updated")),
//             Ok(()),
//             "set from child scope should succeed"
//         );
//         assert_eq!(child.get(&sym), Some(Form::string("updated")),);
//         assert_eq!(
//             parent.borrow().get(&sym),
//             Some(Form::string("updated")),
//             "get should retrieve updated value"
//         );
//     }
// }
