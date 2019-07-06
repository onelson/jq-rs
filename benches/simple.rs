#[macro_use]
extern crate criterion;
extern crate jq_rs;

use criterion::black_box;
use criterion::Criterion;
use jq_rs::{JqProgram, Result};

fn run_one_off(prog: &str, input: &str) -> Result<String> {
    jq_rs::run(prog, input)
}

fn run_pre_compiled(prog: &mut JqProgram, input: &str) -> Result<String> {
    prog.run(input)
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("run one off", |b| {
        b.iter(|| run_one_off(black_box(".name"), black_box(r#"{"name": "John Wick"}"#)))
    });

    c.bench_function("run pre-compiled", |b| {
        let mut prog = jq_rs::compile(".name").unwrap();
        b.iter(|| run_pre_compiled(black_box(&mut prog), black_box(r#"{"name": "John Wick"}"#)))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
