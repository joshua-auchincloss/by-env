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

mod basic_recv {
    use super::envs::*;
    use by_env::env;

    pub struct Basic {}

    #[env(Envs::Dev, Envs::Local)]
    impl Basic {
        #[allow(unused)]
        pub fn test(&self, values: Vec<i32>) -> i32 {
            values.iter().sum()
        }
    }

    #[env(Envs::Qa, Envs::Prod)]
    impl Basic {
        #[allow(unused)]
        /// some doc
        pub fn test(&self, values: Vec<i32>) -> i32 {
            values.iter().sum()
        }
    }
}

fn stp(values: Vec<i32>) -> i32 {
    values.iter().sum()
}

struct Stp {}
impl Stp {
    #[allow(unused)]
    pub fn test(&self, values: Vec<i32>) -> i32 {
        values.iter().sum()
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    use envs::*;

    c.bench_function("cold method through zst", |b| {
        let bs = basic_recv::Basic {};
        b.iter(|| Envs::Dev.impl_basic_test(&bs, black_box(vec![1, 2, 3])))
    });
    c.bench_function("cold through zst", |b| {
        b.iter(|| Envs::Dev.basic_test_fn(black_box(vec![1, 2, 3])))
    });
    c.bench_function("cold with thread_local", |b| {
        with(Envs::Dev, move || {
            b.iter(|| in_context::<Envs, _, _>(|e| e.basic_test_fn(black_box(vec![1, 2, 3]))));
        })
    });
    c.bench_function("cold method with thread_local", |b| {
        with(Envs::Dev, move || {
            let bs = basic_recv::Basic {};

            b.iter(|| {
                in_context::<Envs, _, _>(|e| e.impl_basic_test(&bs, black_box(vec![1, 2, 3])))
            });
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
    c.bench_function("hot method through zst", |b| {
        let bs = basic_recv::Basic {};
        b.iter(|| Envs::Prod.impl_basic_test(&bs, black_box(vec![1, 2, 3])))
    });
    c.bench_function("hot method with thread_local", |b| {
        with(Envs::Dev, move || {
            let bs = basic_recv::Basic {};

            b.iter(|| {
                in_context::<Envs, _, _>(|e| e.impl_basic_test(&bs, black_box(vec![1, 2, 3])))
            });
        })
    });

    c.bench_function("stp", |b| b.iter(|| stp(black_box(vec![1, 2, 3]))));
    c.bench_function("stp method", |b| {
        let bs = Stp {};
        b.iter(|| bs.test(black_box(vec![1, 2, 3])))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
