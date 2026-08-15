use std::io::{self, Read, Write};

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("output") => {
            io::stdout().write_all(b" out\n").unwrap();
            io::stderr().write_all(b" err\n").unwrap();
            std::process::exit(7);
        }
        Some("stdin") => {
            let mut input = Vec::new();
            io::stdin().read_to_end(&mut input).unwrap();
            io::stdout().write_all(&input).unwrap();
        }
        _ => panic!("expected output or stdin"),
    }
}
