use crate::*;
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::parse::Parse;
use syn::{Block, FnArg, Token, Visibility, bracketed};

#[derive(Clone)]
pub struct Stream {
    pub attrs: Vec<syn::Attribute>,

    pub vis: Option<syn::Visibility>,
    pub asy: Option<syn::token::Async>,
    #[allow(unused)]
    pub kw: syn::token::Fn,
    pub generics: Option<syn::Generics>,
    pub name: syn::Ident,
    #[allow(unused)]
    pub paren: syn::token::Paren,
    pub args: syn::punctuated::Punctuated<syn::FnArg, syn::Token![,]>,
    pub ret_sig: Option<syn::Token![->]>,
    pub ret: Option<syn::TypePath>,
    pub content: syn::Block,
}

impl Parse for Stream {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut attrs: Vec<syn::Attribute> = vec![];
        loop {
            if !input.peek(Token![#]) {
                break;
            }
            let inner;
            let attr = syn::Attribute {
                pound_token: input.parse()?,
                style: syn::AttrStyle::Outer,
                bracket_token: bracketed!(inner in input),
                meta: inner.parse()?,
            };
            attrs.push(attr);
        }

        let args;
        Ok(Self {
            attrs,
            vis: peek_parse(input, Token![pub], false)?,
            asy: peek_parse(input, Token![async], false)?,
            kw: input.parse()?,
            name: input.parse()?,
            generics: peek_parse(input, Token![<], false)?,
            paren: syn::parenthesized!(
                args in input
            ),
            args: args.parse_terminated(syn::FnArg::parse, Token![,])?,
            ret_sig: peek_parse(input, Token![->], false)?,
            ret: peek_parse(input, syn::token::Brace, true)?,
            content: input.parse()?,
        })
    }
}

impl Stream {
    pub fn with_injected<I: IntoIterator<Item = FnArg>>(&self, inject: I) -> Self
    where
        <I as IntoIterator>::IntoIter: DoubleEndedIterator,
    {
        let mut out = self.clone();
        for a in inject.into_iter().rev() {
            out.args.insert(0, a)
        }
        out
    }

    pub fn with_content(&mut self, content: Block) {
        self.content = content;
    }

    pub fn with_vis(&mut self, vis: Visibility) {
        self.vis = Some(vis);
    }
}

impl ToTokens for Stream {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.attrs.iter().for_each(|attr| {
            tokens.extend(quote::quote! {#attr});
        });

        maybe_toks(tokens, &self.vis, |vis| quote::quote!(#vis));

        maybe_toks(tokens, &self.asy, |_| quote::quote!(async));

        let iden = &self.name;

        tokens.extend(quote::quote! {
            fn #iden
        });

        maybe_toks(tokens, &self.generics, |g| quote::quote!(#g));

        let args: TokenStream = self.args.iter().map(|arg| quote::quote!(#arg,)).collect();
        tokens.extend(quote::quote!((#args)));

        maybe_toks(tokens, &self.ret_sig, |_| quote::quote!(->));
        maybe_toks(tokens, &self.ret, |ret| quote::quote!(#ret));

        let content = &self.content;

        tokens.extend(quote::quote!(#content));
    }
}

#[cfg(test)]
mod test {
    use super::Stream;

    fn norm_cmp(s: &str) -> String {
        let mut s = s.replace("\n", " ").replace("\t", " ").replace("\r", " ");

        while s.contains("  ") {
            s = s.replace("  ", " ");
        }

        s.trim().into()
    }

    #[test_case::test_case(
        r#"
        # [some_attr]
        # [some_other_attr (inner = "abc")]
        fn test_some_with_attrs () { }
        "#; "parses attributes"
    )]
    #[test_case::test_case(
        r#"
        async fn test_some_with_async () { }
        "#; "parses async function"
    )]
    #[test_case::test_case(
        r#"
        fn some_method (& self ,) { }
        "#; "parses self receiver function"
    )]
    fn round_trip(parse: &str) {
        let strm: Stream = syn::parse_str(parse).unwrap();
        let out = norm_cmp(&quote::quote! {#strm}.to_string());
        assert_eq!(out, norm_cmp(parse))
    }
}
