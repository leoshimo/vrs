//! Main REPL for Lemma
use lemma::lang;
use std::error::Error;
use std::io::{self, Write};

fn main() -> Result<(), Box<dyn Error>> {
    let mut input = String::new();
    let mut env = lang::std_env();

    loop {
        print!("lemma> ");
        io::stdout().flush()?;

        io::stdin().read_line(&mut input)?;
        match lemma::eval_expr(&input, &mut env) {
            Ok(val) => println!("{}", val),
            Err(e) => println!("{}", e),
        }

        input.clear();
    }
}
