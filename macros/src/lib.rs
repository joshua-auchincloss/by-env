pub(crate) mod derive_env;
pub(crate) mod env_attr;
pub(crate) mod fn_stream;
pub(crate) mod function_attr;
pub(crate) mod impl_attr;

use convert_case::{Case, Casing};
use darling::FromDeriveInput;
use proc_macro::TokenStream;
use syn::{Ident, parse::ParseStream};

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

pub(crate) fn macro_name(variant: &Ident) -> Ident {
    ident(format!("define_{}_p", variant.to_string()).to_case(Case::Snake))
}

pub(crate) fn macro_name_nonp(variant: &Ident) -> Ident {
    ident(format!("define_{}_nonp", variant.to_string()).to_case(Case::Snake))
}

pub(crate) fn ident<D: std::fmt::Display>(fmt: D) -> syn::Ident {
    syn::Ident::new(&format!("{fmt}"), proc_macro2::Span::call_site())
}

pub(crate) fn peek_parse<T: syn::parse::Parse, Tok: syn::parse::Peek>(
    input: ParseStream,
    tok: Tok,
    not: bool,
) -> syn::Result<Option<T>> {
    let peek = input.peek(tok);
    let peek = if not { !peek } else { peek };
    if peek {
        Ok(Some(input.parse()?))
    } else {
        Ok(None)
    }
}

pub(crate) fn maybe_toks<T: quote::ToTokens, Get: Fn(&T) -> proc_macro2::TokenStream>(
    tokens: &mut proc_macro2::TokenStream,
    v: &Option<T>,
    get: Get,
) {
    match v {
        Some(v) => tokens.extend(get(v)),
        _ => {}
    }
}

pub(crate) fn try_collect<I: Iterator<Item = Result<T, E>>, T, E>(it: I) -> Result<Vec<T>, E> {
    let mut out = vec![];
    for i in it {
        out.push(i?);
    }
    Ok(out)
}

pub(crate) fn disallow<S: syn::spanned::Spanned, HasDirective: Fn(&T) -> bool, T>(
    spanned: S,
    directive: &str,
    context: &str,
    value: &T,
    has_directive: HasDirective,
) -> syn::Result<()> {
    if has_directive(value) {
        Err(syn::Error::new(
            spanned.span(),
            format!("@{directive} is not allowed in {context} attributes."),
        ))
    } else {
        Ok(())
    }
}
