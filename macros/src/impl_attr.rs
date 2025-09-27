// NOTE: previously imported Deref/DerefMut; no longer needed.

use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    Attribute, FnArg, Generics, Ident, ImplItem, Type, parse::Parse, spanned::Spanned,
    token::Unsafe,
};

use crate::{
    Args, function_attr, ident, resolved_args, swap_self_block, swap_self_receiver, try_collect,
};

#[derive(Clone)]
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

fn handle_name(self_ty: &Type, fn_name: &Ident) -> Ident {
    ident(format!("_impl_{}_{}", self_ty.to_token_stream(), fn_name).to_case(Case::Snake))
}

impl StreamOrPassThru {
    pub fn outer_to_tokens(&self, parent: Args, self_ty: &Type) -> TokenStream {
        match self {
            Self::Stream(ext) => {
                let args = resolved_args(&parent, &ext.fn_args).clone();

                let mut inner_stream = ext.inner.clone();

                inner_stream.name = handle_name(self_ty, &inner_stream.name);

                swap_self_receiver(self_ty, &mut inner_stream.args);
                swap_self_block(&mut inner_stream.content);

                let by_env = function_attr::ByEnv::new(args, inner_stream);

                quote::quote! { #by_env }
            }
            Self::Passthru(pt) => quote::quote!(#pt),
        }
    }
}

pub struct ImplItems {
    pub items: Vec<StreamOrPassThru>,

    attrs: Vec<Attribute>,
    self_ty: Box<Type>,
    unsafety: Option<Unsafe>,
    generics: Generics,
}

// impl Deref for ImplItems {
//     type Target = Vec<StreamOrPassThru>;
//     fn deref(&self) -> &Self::Target {
//         &self.items
//     }
// }

// impl DerefMut for ImplItems {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         &mut self.items
//     }
// }

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
                            if let Some(seg) = attr.meta.path().segments.first()
                                && seg.ident == ident("env")
                            {
                                let args: super::env_attr::ArgsMeta =
                                    syn::parse2(quote::quote! {#attr})?;

                                fn_env.push(args.args);

                                return Ok(None);
                            }

                            Ok(Some(attr.clone()))
                        }))?
                        .into_iter()
                        .flatten()
                        .collect();

                    let fn_env = if fn_env.len() > 1 {
                        return Err(syn::Error::new(
                            span,
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
        Ok(Self {
            items: strm,
            attrs: item.attrs,
            self_ty: item.self_ty,
            unsafety: item.unsafety,
            generics: item.generics,
        })
    }
}

pub struct ByImpl {
    args: Args,
    stream: ImplItems,
}

impl ByImpl {
    pub fn new(args: Args, stream: ImplItems) -> Self {
        let mut stream = stream;
        stream.items.iter_mut().for_each(|it| match it {
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
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let self_ty = &self.stream.self_ty;
        let impl_attrs = &self.stream.attrs;
        let impl_generics = &self.stream.generics; // syn::Generics
        let unsafety = &self.stream.unsafety;

        let mut internal_funcs = TokenStream::new();
        let mut wrapper_items = TokenStream::new();

        for item in &self.stream.items {
            internal_funcs.extend(item.outer_to_tokens(self.args.clone(), self_ty));

            match item {
                StreamOrPassThru::Passthru(..) => {}
                StreamOrPassThru::Stream(ext) => {
                    let mut stream = ext.inner.clone();
                    let args = resolved_args(&self.args, &ext.fn_args).clone();

                    let renamed = handle_name(self_ty, &stream.name);

                    let args_fwd: TokenStream = stream
                        .args
                        .iter()
                        .map(|arg| match arg {
                            FnArg::Receiver(..) => {
                                quote::quote!(self,)
                            }
                            FnArg::Typed(ty) => {
                                let name = &ty.pat;
                                quote::quote! {
                                    #name,
                                }
                            }
                        })
                        .collect();

                    let mut contents = quote::quote! {
                        #renamed(
                            #args_fwd
                        )
                    };

                    if stream.asy.is_some() {
                        contents.extend(quote::quote!(.await))
                    }

                    contents = quote::quote!(
                        {
                            #contents
                        }
                    );

                    stream.with_content(syn::parse2(contents).expect("internal round trip"));

                    for env in args.envs {
                        let macro_name = &env.macro_name;
                        wrapper_items.extend(quote::quote!(
                            #macro_name!{
                                #stream
                            }
                        ));
                    }
                }
            }
        }

        tokens.extend(quote::quote! {
            #internal_funcs

            #(#impl_attrs)*

            #unsafety impl #impl_generics #self_ty {
                #wrapper_items
            }
        });
    }
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
        let this = it.items.iter().next().unwrap();
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
        let this = this.stream.items.iter().next().unwrap();
        match this {
            StreamOrPassThru::Stream(strm) => {
                assert_eq!(strm.envs_as_str(), envs);
            }
            _ => panic!("got passthru"),
        }
    }
}
