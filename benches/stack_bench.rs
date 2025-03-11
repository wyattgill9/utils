use criterion::{Criterion, black_box, criterion_group, criterion_main};
use utils::lfs::stack::LockFreeStack;

fn benchmark_push(c: &mut Criterion) {
    let stack = LockFreeStack::new();
    c.bench_function("lock_free_stack_push", |b| {
        b.iter(|| {
            stack.push(black_box(42));
        })
    });
}

fn benchmark_pop(c: &mut Criterion) {
    let stack = LockFreeStack::new();
    stack.push(42);
    c.bench_function("lock_free_stack_pop", |b| {
        b.iter(|| {
            black_box(stack.pop());
        })
    });
}

criterion_group!(benches, benchmark_push, benchmark_pop);
criterion_main!(benches);
