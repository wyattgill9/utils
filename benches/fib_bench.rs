use criterion::{criterion_group, criterion_main, Criterion};
use utils::math::fib::fib;

fn fib_bench(c: &mut Criterion) {
    c.bench_function("fib 1_000_000", |b| b.iter(|| fib(1_000_000)));
}

criterion_group!(benches, fib_bench);
criterion_main!(benches);
