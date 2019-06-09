#[macro_use]
extern crate criterion;
extern crate json_query;

use criterion::black_box;
use criterion::Criterion;

fn run_one_off(prog: &str, input: &str) -> Result<String, String> {
    json_query::run(prog, input)
}

fn run_pre_compiled(prog: &mut json_query::JqProgram, input: &str) -> Result<String, String> {
    prog.run(input)
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("run one off", |b| {
        b.iter(|| run_one_off(black_box(".name"), black_box(r#"{"name": "John Wick"}"#)))
    });

    c.bench_function("run pre-compiled", |b| {
        let mut prog = json_query::compile(".name").unwrap();
        b.iter(|| run_pre_compiled(black_box(&mut prog), black_box(r#"{"name": "John Wick"}"#)))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
