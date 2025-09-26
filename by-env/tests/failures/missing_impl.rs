//
//
//
//
//
//
//
//
//
//
pub(crate) mod envs {
    use by_env::Env;

    #[derive(Env)]
    pub enum Envs {
        Dev,
        #[env(prod)]
        Prod,
    }
}

mod partial_impl {
    use super::envs::*;
    use by_env::env;

    #[env(Envs::Prod)]
    fn prod_only() -> i64 {
        3
    }
}

fn main() {}
