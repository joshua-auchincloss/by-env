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

mod basic {
    use super::envs::*;
    use by_env::env;

    #[env(Envs::Local, Envs::Dev)]
    pub fn basic_test_fn(a: i32) -> i32 {
        40 + a
    }

    #[env(Envs::Qa, Envs::Prod)]
    pub fn basic_test_fn(a: i32) -> i32 {
        41 + a
    }
}

mod empty_fn {
    use super::envs::*;
    use by_env::env;

    #[env(Envs::Local, Envs::Dev)]
    pub fn empty_fn() {}

    #[env(Envs::Qa, Envs::Prod)]
    pub fn empty_fn() {}
}

mod test {
    use super::*;
    use by_env::with;

    use basic::basic_test_fn;
    use empty_fn::empty_fn;
    use envs::*;

    #[test]
    fn test_basic() {
        assert_eq!(basic_test_fn(1), 42);

        assert_eq!(with(Envs::Local, || { basic_test_fn(1) }), 41);
        assert_eq!(with(Envs::Dev, || { basic_test_fn(1) }), 41);
        assert_eq!(with(Envs::Qa, || { basic_test_fn(1) }), 42);
        assert_eq!(with(Envs::Prod, || { basic_test_fn(1) }), 42);
    }

    #[test]
    fn test_empty() {
        with(Envs::Local, || empty_fn());
        with(Envs::Dev, || empty_fn());
        with(Envs::Qa, || empty_fn());
        with(Envs::Prod, || empty_fn());
    }
}
