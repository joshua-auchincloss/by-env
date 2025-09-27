use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    Attribute, Ident, MetaList, PathSegment, Token, TypePath, bracketed, parenthesized,
    parse::Parse,
    punctuated::Punctuated,
    token::{self, Paren, PathSep},
};

use crate::{
    disallow, fn_stream,
    function_attr::ByEnv,
    impl_attr::{self, ByImpl},
    macro_name, macro_name_nonp,
};

mod kw {
    syn::custom_keyword!(env);
    syn::custom_keyword!(supercede);
}

#[allow(unused)]
pub enum Modifiers {
    Supercede(kw::supercede),
}

impl Parse for Modifiers {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(kw::supercede) {
            Ok(Self::Supercede(input.parse()?))
        } else {
            Err(syn::Error::new(input.span(), "unknown keyword"))
        }
    }
}

#[derive(Clone)]
pub struct Env {
    pub name: Ident,
    pub env: Punctuated<PathSegment, PathSep>,
    pub env_as_holder: Punctuated<PathSegment, PathSep>,
    pub macro_name: Punctuated<PathSegment, PathSep>,
    pub nonp_macro_name: Punctuated<PathSegment, PathSep>,
}

impl ToTokens for Env {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let env = &self.env;
        let name = &self.name;
        tokens.extend(quote::quote! {
            #env::#name
        });
    }
}

impl Parse for Env {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let path: TypePath = input.parse()?;
        let name = path
            .path
            .segments
            .iter()
            .last()
            .expect("fully qualified env path")
            .ident
            .clone();

        let env = path
            .path
            .segments
            .clone()
            .into_iter()
            .take(path.path.segments.len() - 1)
            .collect();

        let root_segment: Punctuated<PathSegment, PathSep> = if path.path.segments.len() > 2 {
            path.path
                .segments
                .clone()
                .into_iter()
                .take(path.path.segments.len() - 2)
                .collect()
        } else {
            Default::default()
        };

        let mut env_as_holder = root_segment.clone();

        env_as_holder.push(PathSegment {
            ident: name.clone(),
            arguments: syn::PathArguments::None,
        });

        let mut qual_macro_name = root_segment.clone();

        qual_macro_name.push(PathSegment {
            ident: macro_name(&name),
            arguments: syn::PathArguments::None,
        });

        let mut nonp_macro_name = root_segment.clone();

        nonp_macro_name.push(PathSegment {
            ident: macro_name_nonp(&name),
            arguments: syn::PathArguments::None,
        });

        Ok(Self {
            name,
            env,
            env_as_holder,
            nonp_macro_name,
            macro_name: qual_macro_name,
        })
    }
}

#[derive(Clone)]
pub struct Args {
    pub envs: Punctuated<Env, Token![,]>,
    pub supercede: bool,
}

impl Args {
    pub fn merge(&mut self, other: &Args) {
        if !self.supercede && !other.supercede {
            self.envs.extend(other.envs.clone());
        } else if other.supercede {
            self.envs = other.envs.clone();
        };

        let mut seen = vec![];

        self.envs = self
            .envs
            .iter()
            .filter(|it| {
                if seen.contains(&it.name) {
                    false
                } else {
                    seen.push(it.name.clone());
                    true
                }
            })
            .map(Clone::clone)
            .collect();
    }
}

impl From<Args> for Attribute {
    fn from(val: Args) -> Self {
        let envs = &val.envs;
        Attribute {
            pound_token: token::Pound::default(),
            style: syn::AttrStyle::Outer,
            bracket_token: token::Bracket::default(),
            meta: syn::Meta::List(MetaList {
                path: syn::parse2(quote::quote! {}).expect("empty"),
                delimiter: syn::MacroDelimiter::Paren(Paren::default()),
                tokens: quote::quote!(#envs),
            }),
        }
    }
}

impl Parse for Args {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut supercede = false;
        if input.peek(Token![@]) {
            loop {
                if !input.peek(Token![@]) {
                    break;
                }
                let _: Token![@] = input.parse()?;
                match input.parse()? {
                    Modifiers::Supercede(..) => {
                        supercede = true;
                    }
                }
            }
        };

        let envs = input.parse_terminated(Env::parse, Token![,])?;

        Ok(Self { envs, supercede })
    }
}

pub struct ArgsMeta {
    #[allow(unused)]
    pound: Token![#],
    #[allow(unused)]
    brack: syn::token::Bracket,
    #[allow(unused)]
    env: kw::env,
    #[allow(unused)]
    paren: syn::token::Paren,
    pub args: Args,
}

impl Parse for ArgsMeta {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let brack;
        let paren;
        Ok(Self {
            pound: input.parse()?,
            brack: bracketed!(brack in input),
            env: brack.parse()?,
            paren: parenthesized!(paren in brack),
            args: paren.parse()?,
        })
    }
}

#[allow(clippy::large_enum_variant)]
pub enum EnvStreams {
    Impl(impl_attr::ImplItems),
    Function(fn_stream::Stream),
}

impl Parse for EnvStreams {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(token::Impl) {
            Ok(Self::Impl(input.parse()?))
        } else {
            Ok(Self::Function(input.parse()?))
        }
    }
}

pub enum EnvAttr {
    Impl(ByImpl),
    Function(Box<ByEnv>),
}

impl ToTokens for EnvAttr {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Impl(strc) => strc.to_tokens(tokens),
            Self::Function(env) => env.to_tokens(tokens),
        }
    }
}

impl EnvAttr {
    pub fn new(atts: TokenStream, stream: TokenStream) -> Result<Self, syn::Error> {
        let args: Args = syn::parse2(atts)?;
        let stream: EnvStreams = syn::parse2(stream)?;
        Ok(match stream {
            EnvStreams::Function(func) => {
                disallow(&args.envs, "supercede", "fn", &args, |a| a.supercede)?;
                Self::Function(Box::new(ByEnv::new(args, func)))
            }
            EnvStreams::Impl(strc) => {
                disallow(&args.envs, "supercede", "impl", &args, |a| a.supercede)?;
                Self::Impl(ByImpl::new(args, strc))
            }
        })
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use proc_macro2::TokenStream;

    use quote::ToTokens;

    use crate::{Args, env_attr::EnvAttr, ident};

    fn assert_to_str<S: ToTokens>(s: S, expect: &str, desc: &str) {
        assert_eq!(
            quote::quote!(#s).to_string().replace(" ", ""),
            expect,
            "{}",
            desc
        )
    }

    #[test_case::test_case("Env::Local", "Env", "Local", "Local", "define_local_p"; "local paths are respected")]
    #[test_case::test_case(
        "some_crate::env::Env::Local",
        "some_crate::env::Env",
        "Local",
        "some_crate::env::Local",
        "some_crate::env::define_local_p"
        ; "crate paths are respected"
    )]
    fn env_args(from_str: &str, parent: &str, name: &str, struct_variant: &str, macro_name: &str) {
        let e: super::Env = syn::parse_str(from_str).unwrap();

        assert_to_str(&e.env, parent, "root crate path must equal parsed path");

        assert_eq!(e.name, ident(name), "parsed env variant must equal name");

        assert_to_str(
            &e.env_as_holder,
            struct_variant,
            "parsed env variant must point to struct method holder",
        );

        assert_to_str(
            &e.macro_name,
            macro_name,
            "parsed env variant must point to macro definition",
        );
    }

    #[test_case::test_case("#[env(Envs::Dev, Envs::Local)]"; "parses full form")]
    #[test_case::test_case("#[env(Envs::Dev, Envs::Local,)]"; "ignores trailing comma")]
    #[test_case::test_case("#[env(@supercede Envs::Dev, Envs::Local)]"; "parses directives")]
    fn test_args_meta(parse: &str) {
        let _: super::ArgsMeta = syn::parse_str(parse).unwrap();
    }

    #[test_case::test_case(
        "Envs::Dev",
        "
        impl Abc {
            #[env(Envs::Prod)]
            fn test(&self) {}
        }
        "; "impl form"
    )]
    fn test_impl_form(atts: &str, stream: &str) {
        let atts = TokenStream::from_str(atts).unwrap();
        let stream = TokenStream::from_str(stream).unwrap();
        let attr = EnvAttr::new(atts, stream).expect("attr");
        assert!(matches!(attr, EnvAttr::Impl(..)));
    }

    #[test_case::test_case(
        "Envs::Dev",
        "
        fn test(a: i32) -> i32 { 42 }
        "; "basic function form"
    )]
    fn test_fn_form(atts: &str, stream: &str) {
        let atts = TokenStream::from_str(atts).unwrap();
        let stream = TokenStream::from_str(stream).unwrap();
        let attr = EnvAttr::new(atts, stream).expect("func");
        assert!(matches!(attr, EnvAttr::Function(..)));
    }

    #[test_case::test_case(
        "D", "@supercede A, B, C", true, vec!["A", "B", "C"]; "this supercedes"
    )]
    #[test_case::test_case(
        "@supercede D", "A, B, C", false, vec!["D"]; "other supercedes"
    )]
    #[test_case::test_case(
        "D", "A, B, C", false, vec!["A", "B", "C", "D"]; "does not supercede"
    )]
    fn test_merge(parent: &str, this: &str, this_supercede: bool, expect: Vec<&str>) {
        let parent: Args = syn::parse_str(parent).unwrap();

        let mut this: Args = syn::parse_str(this).unwrap();
        assert_eq!(this.supercede, this_supercede);

        this.merge(&parent);

        let out: Vec<_> = this.envs.iter().map(|it| it.name.to_string()).collect();

        assert_eq!(out, expect)
    }
}
