extern crate jq_rs;
use std::env;

fn main() {
    let mut args = env::args().skip(1);

    let program = args.next().unwrap();
    let input = args.next().unwrap();
    match jq_rs::run(&program, &input) {
        Ok(s) => println!("{}", s),
        Err(e) => eprintln!("{}", e),
    }
}
