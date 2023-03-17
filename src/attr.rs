use crate::meta::MetaItem;
use proc_macro2::Span;
use syn::{parse::Parse, spanned::Spanned};

mod kw {
    use syn::custom_keyword;

    custom_keyword!(source);
    custom_keyword!(into);
    custom_keyword!(iter_into);
    custom_keyword!(map_into);
    custom_keyword!(map);
    custom_keyword!(skip);
}

pub enum Attr {
    Source(syn::Path),

    Into,
    IterInto,
    MapInto,

    Map(syn::Expr),
    Skip,
}

macro_rules! parse_attrs {
    (kv: $k:path, $v:path, $var:ident, $inner:ident) => {
        if $inner.peek($k) {
            return MetaItem::<$k, $v>::parse(&$inner).map(|m| Self::$var(m.into_value()));
        }
    };
    (flag: $t:path, $var:ident, $inner:ident) => {
        if $inner.peek($t) {
            let _: $t = $inner.parse()?;
            return Ok(Self::$var);
        }
    };
    ($(($($t:tt)*)),*) => {
        impl Parse for Attr {
            fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
                let inner;
                syn::parenthesized!(inner in input);

                $(
                    parse_attrs!($($t)*, inner);
                )*

                Err(syn::Error::new(
                    inner.span(),
                    "Invalid #[from(...)] attribute"
                ))
            }
        }
    }
}

parse_attrs!(
    (kv: kw::source, syn::Path, Source),
    (flag: kw::into, Into),
    (flag: kw::iter_into, IterInto),
    (flag: kw::map_into, MapInto),
    (kv: kw::map, syn::Expr, Map),
    (flag: kw::skip, Skip)
);

impl Attr {
    pub fn collect(attrs: &[syn::Attribute]) -> Vec<(Span, syn::Result<Self>)> {
        attrs
            .iter()
            .filter(|attr| {
                attr.path
                    .get_ident()
                    .map(|ident| ident == "from")
                    .unwrap_or(false)
            })
            .map(|attr| (attr.span(), syn::parse2::<Self>(attr.tokens.clone())))
            .collect()
    }
}
