use std::ops::{Deref, DerefMut};

use quote::ToTokens;
use syn::{
    ImplItem,
    parse::Parse, spanned::Spanned,
};

use crate::{Args, ident, try_collect};

pub struct StreamExt {
    fn_args: Option<Args>,
    inner: super::fn_stream::Stream,
}

#[cfg(test)]
impl StreamExt {
    fn envs_as_str(&self) -> Vec<String> {
        self.fn_args
            .clone()
            .map(|ok| ok.envs.iter().map(|env| env.name.to_string()).collect())
            .unwrap_or_default()
    }
}

pub enum StreamOrPassThru {
    Stream(StreamExt),
    Passthru(ImplItem),
}

pub struct ImplItems(Vec<StreamOrPassThru>);

impl Deref for ImplItems {
    type Target = Vec<StreamOrPassThru>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ImplItems {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Parse for ImplItems {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let item: syn::ItemImpl = input.parse()?;

        let strm: Vec<_> = try_collect(item.items.into_iter().map(|it| -> syn::Result<_> {
            match it {
                ImplItem::Fn(f) => {
                    let mut fn_env = vec![];
                    let span = f.span();
                    let attrs: Vec<_> =
                        try_collect(f.attrs.clone().iter().map(|attr| -> syn::Result<_> {
                            if let Some(seg) = attr.meta.path().segments.first() {
                                if seg.ident == ident("env") {
                                    let args: super::env_attr::ArgsMeta =
                                        syn::parse2(quote::quote! {#attr})?;

                                    fn_env.push(args.args);

                                    return Ok(None);
                                }
                            }

                            Ok(Some(attr.clone()))
                        }))?
                        .into_iter()
                        .filter_map(|it| it)
                        .collect();

                    let fn_env = if fn_env.len() > 1 {
                        return Err(syn::Error::new(
                            span.clone(),
                            "item impls may only have one #[env(..)] specification.",
                        ));
                    } else {
                        fn_env.into_iter().next()
                    };

                    let mut stream: super::fn_stream::Stream = syn::parse2(quote::quote!(#f))?;
                    stream.attrs = attrs;

                    Ok(StreamOrPassThru::Stream(StreamExt {
                        fn_args: fn_env,
                        inner: stream,
                    }))
                }
                st => Ok(StreamOrPassThru::Passthru(st)),
            }
        }))?;
        Ok(Self(strm))
    }
}

pub struct ByImpl {
    args: Args,
    stream: ImplItems,
}

impl ByImpl {
    pub fn new(args: Args, stream: ImplItems) -> Self {
        let mut stream = stream;
        stream.iter_mut().for_each(|it| match it {
            StreamOrPassThru::Passthru(..) => {}
            StreamOrPassThru::Stream(strm) => {
                if let Some(ok) = &mut strm.fn_args {
                    ok.merge(&args);
                }
            }
        });
        Self { args, stream }
    }
}

impl ToTokens for ByImpl {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {}
}

#[cfg(test)]
mod test {
    use super::*;

    #[test_case::test_case(
        "
        impl T { 
            fn test() {}
        }", Vec::<&str>::default(); "basic no attributes"
    )]
    #[test_case::test_case(
        "
        impl T { 
            #[env(Envs::Dev, Envs::Local)]
            fn test() {}
        }", vec!["Dev", "Local"]; "basic fn attributes"
    )]
    fn test_parse_member(parse: &str, envs: Vec<&str>) {
        let it: ImplItems = syn::parse_str(parse).expect("parse");
        let this = it.iter().next().unwrap();
        match this {
            StreamOrPassThru::Stream(strm) => {
                assert_eq!(strm.envs_as_str(), envs);
            }
            _ => panic!("got passthru"),
        }
    }

    #[test_case::test_case(
        "#[env(Envs::Prod)]", 
        "
        impl T { 
            #[env(@supercede Envs::Dev, Envs::Local)]
            fn test() {}
        }", vec!["Dev", "Local"]; "supercedes impl envs"
    )]
    #[test_case::test_case(
        "#[env(Envs::Prod)]", 
        "
        impl T { 
            #[env(Envs::Dev, Envs::Local)]
            fn test() {}
        }", vec!["Dev", "Local", "Prod"]; "merges impl envs"
    )]
    fn test_env_resolver(args: &str, stream: &str, envs: Vec<&str>) {
        let args: crate::env_attr::ArgsMeta = syn::parse_str(args).expect("parse");
        let stream: ImplItems = syn::parse_str(stream).expect("parse");
        let this = super::ByImpl::new(args.args, stream);
        let this = this.stream.iter().next().unwrap();
        match this {
            StreamOrPassThru::Stream(strm) => {
                assert_eq!(strm.envs_as_str(), envs);
            }
            _ => panic!("got passthru"),
        }
    }
}
