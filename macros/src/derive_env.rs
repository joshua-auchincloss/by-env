use proc_macro2::TokenStream;
use syn::Ident;

use crate::{macro_name, macro_name_nonp};

fn no() -> bool {
    false
}

#[derive(darling::FromVariant)]
#[darling(attributes(env))]
pub struct Field {
    ident: Ident,
    #[darling(default = "no")]
    prod: bool,
}

#[derive(darling::FromDeriveInput)]
#[darling(attributes(env))]
pub struct Envs {
    ident: Ident,
    data: darling::ast::Data<Field, darling::util::Ignored>,
}

impl Envs {
    pub fn derive_env(self) -> TokenStream {
        let env = &self.ident;

        let mut tt = quote::quote!();
        let mut by_env = quote::quote!();
        let mut env_use = quote::quote!();
        let mut env_match = quote::quote! {};
        let mut env_match_async = quote::quote! {};

        let envs = self.data.take_enum().unwrap();

        if envs.iter().filter(|it| it.prod).count() != 1 {
            return syn::Error::new(
                self.ident.span(),
                "exactly one prod environment must be specified. \
                        this is used as the source of origin for type signatures for compatibility.",
            )
            .into_compile_error();
        }

        for field in &envs {
            let name = &field.ident;

            tt.extend(quote::quote!(
                pub struct #name;

                impl From<#name> for #env {
                    fn from(_: #name) -> Self {
                        Self::#name
                    }
                }
            ));

            env_match.extend(quote::quote! {
                Self::#name => #name::$f($($($arg,)*)?),
            });
            env_match_async.extend(quote::quote! {
                Self::#name => #name::$f($($($arg,)*)?).await,
            });

            let macro_name = macro_name(name);
            let nonp_macro = macro_name_nonp(name);

            env_use.extend(quote::quote!(
                pub(crate) use __by_env::#macro_name;
                pub(crate) use __by_env::#nonp_macro;
            ));

            if field.prod {
                by_env.extend(quote::quote! {
                    #[allow(unused)]
                    macro_rules! #macro_name {
                        ($block: item) => {
                            $block
                        }
                    }
                    pub(crate) use #macro_name;

                    #[allow(unused)]
                    macro_rules! #nonp_macro {
                        ($block: item) => {}
                    }
                    pub(crate) use #nonp_macro;
                });

                tt.extend(quote::quote! {
                    thread_local!{
                        static ENV: std::cell::RefCell<#env> = std::cell::RefCell::new(<#env as by_env::Env>::prod());
                    }

                    impl by_env::Env for #env {
                        fn prod() -> Self {
                            Self::#name
                        }

                        fn key() -> &'static std::thread::LocalKey<std::cell::RefCell<Self>> {
                            &ENV
                        }
                    }
                })
            } else {
                by_env.extend(quote::quote! {
                    #[allow(unused)]
                    macro_rules! #macro_name {
                        ($block: item) => {}
                    }
                    pub(crate) use #macro_name;

                    #[allow(unused)]
                    macro_rules! #nonp_macro {
                        ($block: item) => {
                            $block
                        }
                    }
                    pub(crate) use #nonp_macro;
                });
            }
        }

        tt.extend(quote::quote! {
            pub(crate) mod __by_env {
                #by_env

                #[allow(unused)]
                macro_rules! __match_env {
                    ($this: ident -> $f: ident ($($($arg: ident), + $(,)?)?)) => {
                        match $this {
                            #env_match
                        }
                    };
                    (async $this: ident -> $f: ident ($($($arg: ident), + $(,)?)?)) => {
                        match $this {
                            #env_match_async
                        }
                    }
                }

                pub(crate) use __match_env;
            }
            #env_use
            pub(crate) use __by_env::__match_env;
        });

        tt
    }
}
