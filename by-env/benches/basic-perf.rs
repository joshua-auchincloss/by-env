use std::hint::black_box;

use by_env::{in_context, with};
use criterion::{Criterion, criterion_group, criterion_main};
pub(crate) mod envs {
    use by_env::Env;

    #[derive(Env)]
    pub enum Envs {
        Local,
        Dev,
        Qa,
        #[env(prod)]
        Prod,
    }
}

pub use envs::Envs;

mod basic {
    use super::envs::*;
    use by_env::env;

    #[env(Envs::Local, Envs::Dev)]
    pub fn basic_test_fn(values: Vec<i32>) -> i32 {
        values.iter().sum()
    }

    #[env(Envs::Qa, Envs::Prod)]
    pub fn basic_test_fn(values: Vec<i32>) -> i32 {
        values.iter().sum()
    }
}

fn stp(values: Vec<i32>) -> i32 {
    values.iter().sum()
}

fn criterion_benchmark(c: &mut Criterion) {
    use envs::*;

    c.bench_function("cold through zst", |b| {
        b.iter(|| Envs::Dev.basic_test_fn(black_box(vec![1, 2, 3])))
    });
    c.bench_function("cold with thread_local", |b| {
        with(Envs::Dev, move || {
            b.iter(|| in_context::<Envs, _, _>(|e| e.basic_test_fn(black_box(vec![1, 2, 3]))));
        })
    });
    c.bench_function("hot through zst", |b| {
        b.iter(|| Envs::Prod.basic_test_fn(black_box(vec![1, 2, 3])))
    });
    c.bench_function("hot with thread_local", |b| {
        with(Envs::Prod, move || {
            b.iter(|| in_context::<Envs, _, _>(|e| e.basic_test_fn(black_box(vec![1, 2, 3]))));
        })
    });
    c.bench_function("stp", |b| b.iter(|| stp(black_box(vec![1, 2, 3]))));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
