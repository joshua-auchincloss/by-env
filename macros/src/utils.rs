use convert_case::{Case, Casing};
use syn::{Block, Ident, Stmt, parse::ParseStream, spanned::Spanned, token::Colon};

use crate::Args;

pub fn macro_name(variant: &Ident) -> Ident {
    ident(format!("define_{}_p", variant).to_case(Case::Snake))
}

pub fn macro_name_nonp(variant: &Ident) -> Ident {
    ident(format!("define_{}_nonp", variant).to_case(Case::Snake))
}

pub fn ident<D: std::fmt::Display>(fmt: D) -> syn::Ident {
    syn::Ident::new(&format!("{fmt}"), proc_macro2::Span::call_site())
}

pub fn peek_parse<T: syn::parse::Parse, Tok: syn::parse::Peek>(
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

pub fn maybe_toks<T: quote::ToTokens, Get: Fn(&T) -> proc_macro2::TokenStream>(
    tokens: &mut proc_macro2::TokenStream,
    v: &Option<T>,
    get: Get,
) {
    if let Some(v) = v {
        tokens.extend(get(v))
    }
}

pub fn try_collect<I: Iterator<Item = Result<T, E>>, T, E>(it: I) -> Result<Vec<T>, E> {
    let mut out = vec![];
    for i in it {
        out.push(i?);
    }
    Ok(out)
}

pub fn disallow<S: syn::spanned::Spanned, HasDirective: Fn(&T) -> bool, T>(
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

/// self                => slf: <SelfType>
/// mut self            => mut slf: <SelfType>
/// &self               => slf: &<SelfType>
/// &'a self            => slf: &'a <SelfType>
/// &mut self           => slf: &mut <SelfType>
/// &'a mut self        => slf: &'a mut <SelfType>
/// self: Arc<Self>     => slf: Arc<...SelfType...>
/// mut self: Box<Self> => mut slf: Box<...SelfType...>
pub fn swap_self_receiver(
    self_ty: &syn::Type,
    args: &mut syn::punctuated::Punctuated<syn::FnArg, syn::token::Comma>,
) -> Option<syn::Ident> {
    use syn::{FnArg, Type, TypePath};

    fn substitute_self(ty: &mut Type, new_ty: &Type) {
        match ty {
            Type::Path(TypePath { qself: None, path }) if path.segments.len() == 1 => {
                if path.segments.first().unwrap().ident == ident("Self") {
                    *ty = new_ty.clone();
                }
            }
            Type::Reference(r) => substitute_self(&mut r.elem, new_ty),
            Type::Array(a) => substitute_self(&mut a.elem, new_ty),
            Type::Group(g) => substitute_self(&mut g.elem, new_ty),
            Type::Paren(p) => substitute_self(&mut p.elem, new_ty),
            Type::Ptr(p) => substitute_self(&mut p.elem, new_ty),
            Type::Slice(s) => substitute_self(&mut s.elem, new_ty),
            Type::Tuple(t) => {
                for elem in t.elems.iter_mut() {
                    substitute_self(elem, new_ty);
                }
            }
            _ => {}
        }
    }

    let first = args.first_mut()?;

    let recv = match first {
        FnArg::Receiver(r) => r.clone(), // work on a clone, then replace
        _ => return None,
    };

    let new_type: Type = if recv.colon_token.is_some() {
        // self: <T>
        let mut ty: Type = recv.ty.as_ref().clone();
        substitute_self(&mut ty, self_ty);
        ty
    } else if let Some((_, lifetime)) = &recv.reference {
        // & '? mut? self
        Type::Reference(syn::TypeReference {
            and_token: syn::token::And(recv.self_token.span()),
            lifetime: lifetime.clone(),
            mutability: recv.mutability,
            elem: Box::new(self_ty.clone()),
        })
    } else {
        // self
        self_ty.clone()
    };

    // mut self
    let pat_mut = if recv.mutability.is_some() && recv.reference.is_none() {
        Some(syn::token::Mut(recv.self_token.span()))
    } else {
        None
    };

    // ident(..) could be used here, but we want to respect spans
    let ident_slf = syn::Ident::new("slf", recv.self_token.span());
    let pat: syn::Pat = syn::Pat::Ident(syn::PatIdent {
        attrs: vec![],
        by_ref: None,
        mutability: pat_mut,
        ident: ident_slf.clone(),
        subpat: None,
    });

    let pat_ty = syn::PatType {
        attrs: vec![],
        pat: Box::new(pat),
        colon_token: Colon(recv.self_token.span()),
        ty: Box::new(new_type),
    };

    *first = FnArg::Typed(pat_ty);
    Some(ident_slf)
}

pub fn swap_self_block(block: &mut Block) {
    use syn::{Expr, Pat};

    fn rewrite_path(path: &mut syn::Path) {
        if path.segments.len() == 1 {
            let seg = path.segments.first_mut().unwrap();
            if seg.ident == crate::ident("self") {
                let span = seg.ident.span();
                seg.ident = syn::Ident::new("slf", span);
            }
        }
    }

    fn visit_expr(expr: &mut Expr) {
        match expr {
            Expr::Path(p) => {
                if p.qself.is_none() {
                    rewrite_path(&mut p.path);
                }
            }
            Expr::MethodCall(mc) => {
                visit_expr(&mut mc.receiver);
                mc.args.iter_mut().for_each(visit_expr);
            }
            Expr::Field(f) => visit_expr(&mut f.base),
            Expr::Call(c) => {
                visit_expr(&mut c.func);
                c.args.iter_mut().for_each(visit_expr);
            }
            Expr::Block(b) => b.block.stmts.iter_mut().for_each(visit_stmt),
            Expr::If(i) => {
                visit_expr(&mut i.cond);
                i.then_branch.stmts.iter_mut().for_each(visit_stmt);
                if let Some((_, else_expr)) = &mut i.else_branch {
                    visit_expr(else_expr);
                }
            }
            Expr::Match(m) => {
                visit_expr(&mut m.expr);
                for arm in &mut m.arms {
                    visit_expr(&mut arm.body);
                }
            }
            Expr::While(w) => {
                visit_expr(&mut w.cond);
                w.body.stmts.iter_mut().for_each(visit_stmt);
            }
            Expr::Loop(l) => l.body.stmts.iter_mut().for_each(visit_stmt),
            Expr::ForLoop(f) => {
                visit_expr(&mut f.expr);
                f.body.stmts.iter_mut().for_each(visit_stmt);
            }
            Expr::Unary(u) => visit_expr(&mut u.expr),
            Expr::Paren(p) => visit_expr(&mut p.expr),
            Expr::Reference(r) => visit_expr(&mut r.expr),
            Expr::Return(r) => {
                if let Some(e) = &mut r.expr {
                    visit_expr(e);
                }
            }
            Expr::Await(a) => visit_expr(&mut a.base),
            Expr::Try(t) => visit_expr(&mut t.expr),
            Expr::Closure(c) => visit_expr(&mut c.body),
            _ => {}
        }
    }

    fn visit_pat(pat: &mut Pat) {
        match pat {
            Pat::Ident(id) => {
                if id.ident == crate::ident("self") {
                    let span = id.ident.span();
                    id.ident = syn::Ident::new("slf", span);
                }
            }
            Pat::Tuple(tp) => tp.elems.iter_mut().for_each(visit_pat),
            Pat::Slice(sl) => sl.elems.iter_mut().for_each(visit_pat),
            Pat::Reference(r) => visit_pat(&mut r.pat),
            Pat::Or(or) => or.cases.iter_mut().for_each(visit_pat),
            _ => {}
        }
    }

    fn visit_stmt(stmt: &mut Stmt) {
        match stmt {
            Stmt::Local(local) => {
                visit_pat(&mut local.pat);
                if let Some(init) = &mut local.init {
                    visit_expr(&mut init.expr);
                }
            }
            Stmt::Item(_) => {}
            Stmt::Expr(expr, opt_semi) => {
                let _ = opt_semi; // preserve possible second field (syn 2 shape)
                visit_expr(expr)
            }
            Stmt::Macro(_) => {}
        }
    }

    block.stmts.iter_mut().for_each(visit_stmt);
}

pub fn resolved_args<'a>(parent: &'a Args, fn_args: &'a Option<Args>) -> &'a Args {
    if let Some(overrides) = fn_args.as_ref() {
        overrides
    } else {
        parent
    }
}
