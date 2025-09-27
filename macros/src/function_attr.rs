use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{FnArg, Ident, Pat, Visibility};

use crate::{env_attr::Args, fn_stream::Stream, ident};

fn name_of_arg(arg: &FnArg) -> Ident {
    match arg {
        FnArg::Receiver(..) => ident("self".to_string()),
        FnArg::Typed(ty) => {
            if let Pat::Ident(i) = ty.pat.as_ref() {
                i.ident.clone()
            } else {
                panic!("expected ident")
            }
        }
    }
}

pub struct ByEnv {
    args: Args,
    stream: Stream,
}

impl ByEnv {
    pub fn new(args: Args, stream: Stream) -> Self {
        Self { args, stream }
    }
}

impl ToTokens for ByEnv {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let fname = &self.stream.name;
        let env = &self.args.envs.iter().next().unwrap().env;

        let mut strm = self.stream.clone();

        // we are overriding here for the generated environment members
        strm.with_vis(Visibility::Public(syn::token::Pub(strm.name.span())));

        let fn_args: TokenStream = self
            .stream
            .args
            .iter()
            .map(|arg| {
                let name = name_of_arg(arg);
                quote::quote!(
                    #name,
                )
            })
            .collect();

        let mut as_ctx = self.stream.clone();
        as_ctx.with_content(
            syn::parse2(if as_ctx.asy.is_some() {
                quote::quote! {
                    {
                        by_env::in_async_context::<#env, _, _>(|ctx| ctx.#fname(#fn_args)).await
                    }
                }
            } else {
                quote::quote! {
                    {
                        by_env::in_context::<#env, _, _>(|ctx| ctx.#fname(#fn_args))
                    }
                }
            })
            .expect("ctx"),
        );

        // we keep vis the same here to respect public / private apis
        let mut by_env = self
            .stream
            .with_injected(vec![syn::parse2(quote::quote!(&self)).expect("env as arg")]);

        // invoke the generated matcher derived by derive(Env)
        // be careful of async values
        by_env.with_content(
            syn::parse2(if self.stream.asy.is_some() {
                quote::quote! {
                    {
                        let this = &self;
                        __match_env! {
                            async this -> #fname (#fn_args)
                        }
                    }
                }
            } else {
                quote::quote! {
                    {
                        let this = &self;
                        __match_env! {
                            this -> #fname (#fn_args)
                        }
                    }
                }
            })
            .expect("content"),
        );

        self.args.envs.iter().for_each(|env| {
            let env_name = &env.env_as_holder;
            let macro_name = &env.macro_name;
            let env_ty = &env.env;
            let nonp_macro = &env.nonp_macro_name;

            tokens.extend(quote::quote!(
                impl #env_name {
                    #nonp_macro!{
                        #[cold]
                        #[inline]
                        #strm
                    }
                    #macro_name!{
                        #[inline(always)]
                        #strm
                    }
                }

                impl #env_ty {
                    #macro_name!{
                        #[inline(always)]
                        #by_env
                    }
                }

                #macro_name!{
                    #as_ctx
                }
            ))
        });
    }
}
