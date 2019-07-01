extern crate json_query;
use std::env;

fn main() {
    let mut args = env::args().skip(1);

    let program = args.next().unwrap();
    let input = args.next().unwrap();
    match json_query::run(&program, &input) {
        Ok(s) => println!("{}", s),
        Err(e) => eprintln!("{}", e),
    }
}
