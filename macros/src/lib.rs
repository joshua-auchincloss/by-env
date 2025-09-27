pub(crate) mod derive_env;
pub(crate) mod env_attr;
pub(crate) mod fn_stream;
pub(crate) mod function_attr;
pub(crate) mod impl_attr;
pub(crate) mod utils;

use darling::FromDeriveInput;
use proc_macro::TokenStream;
pub(crate) use utils::*;

use crate::env_attr::Args;

macro_rules! bail {
    ($out: expr) => {
        match $out {
            Ok(val) => val,
            Err(e) => return e.into_compile_error().into(),
        }
    };
}

#[proc_macro_attribute]
pub fn env(atts: TokenStream, stream: TokenStream) -> TokenStream {
    let env = bail!(env_attr::EnvAttr::new(atts.into(), stream.into()));
    quote::quote!(#env).into()
}

#[proc_macro_derive(Env, attributes(env))]
pub fn derive_env(atts: TokenStream) -> TokenStream {
    match derive_env::Envs::from_derive_input(&bail!(syn::parse(atts))) {
        Ok(der) => der,
        Err(e) => return e.write_errors().into(),
    }
    .derive_env()
    .into()
}
