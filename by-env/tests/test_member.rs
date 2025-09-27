use by_env::with;

use crate::envs::Envs;

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

    pub struct Basic {
        pub a: usize,
    }

    #[env(Envs::Dev, Envs::Local)]
    impl Basic {
        #[allow(unused)]
        pub fn test(&self) -> usize {
            0
        }
    }

    #[env(Envs::Qa, Envs::Prod)]
    impl Basic {
        /// some doc
        pub fn test(&self) -> usize {
            self.a
        }
    }
}

#[test]
fn test() {
    let b = basic_recv::Basic { a: 42 };

    assert_eq!(with(Envs::Local, || b.test()), 0);
    assert_eq!(with(Envs::Dev, || b.test()), 0);
    assert_eq!(with(Envs::Qa, || b.test()), 42);
    assert_eq!(with(Envs::Prod, || b.test()), 42);
}
