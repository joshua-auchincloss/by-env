pub(crate) mod envs {
    use by_env::Env;

    #[derive(Env)]
    pub enum Envs {
        Dev,
        #[env(prod)]
        Prod,
    }
}

mod basic_with_generic {
    use crate::envs::*;
    use by_env::env;

    #[env(Envs::Prod)]
    pub fn basic_with_generic<G: std::ops::Add<Output = G> + std::ops::Add<i32, Output = G>>(
        a: G,
        b: G,
    ) -> G {
        a + b
    }

    #[env(Envs::Dev)]
    pub fn basic_with_generic<G: std::ops::Add<Output = G> + std::ops::Add<i32, Output = G>>(
        a: G,
        b: G,
    ) -> G {
        a + b + 3
    }
}

mod test {
    use by_env::with;

    use crate::envs::Envs;

    #[test]
    fn test() {
        with(Envs::Dev, || {
            assert_eq!(super::basic_with_generic::basic_with_generic(1, 2), 6);
        });

        with(Envs::Prod, || {
            assert_eq!(super::basic_with_generic::basic_with_generic(1, 2), 3);
        });
    }
}
