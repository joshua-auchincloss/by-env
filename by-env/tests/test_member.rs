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

mod basic_recv {
    use super::envs::*;
    use by_env::env;

    struct Basic {
        a: usize,
    }

    #[env(Envs::Dev, Envs::Local)]
    impl Basic {
        #[env(Envs::Qa, Envs::Prod)]
        fn test() {}
    }
}

#[test]
fn test() {}
