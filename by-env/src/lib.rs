use std::cell::RefCell;

pub use by_env_macros::{Env, env};

pub trait Env: Sized + 'static {
    fn prod() -> Self;
    fn key() -> &'static std::thread::LocalKey<RefCell<Self>>;
}

pub fn with<E: Env, F: FnOnce() -> R, R>(env: E, f: F) -> R {
    let key = E::key();
    let before = key.replace(env);
    let r = f();
    key.set(before);
    r
}

pub fn in_context<E: Env, F: FnOnce(&E) -> R, R>(f: F) -> R {
    let key = E::key();
    key.with_borrow(move |v| f(v))
}

pub async fn in_async_context<E: Env, F: FnOnce(&E) -> Fut, R, Fut: Future<Output = R>>(f: F) -> R {
    let key = E::key();
    key.with_borrow(move |v| f(v)).await
}

pub trait Prod {}
