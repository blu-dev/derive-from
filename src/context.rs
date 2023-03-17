use crate::attr::Attr;
use proc_macro2::{Span, TokenStream};
use proc_macro_error::{abort, emit_error};
use quote::quote;
use syn::{spanned::Spanned, Data, DataStruct, DeriveInput, Fields};

enum ConversionMethod {
    None,
    Into,
    IterInto,
    MapInto,
    Map(syn::Expr),
    Skip,
}

impl ConversionMethod {
    pub fn try_replace(&mut self, value: Attr, span: Span) {
        if !matches!(self, Self::None) {
            emit_error!(span, "only use one conversion method for a field");
        }

        match value {
            Attr::Into => *self = Self::Into,
            Attr::IterInto => *self = Self::IterInto,
            Attr::MapInto => *self = Self::MapInto,
            Attr::Map(func) => *self = Self::Map(func),
            Attr::Skip => *self = Self::Skip,
            _ => {
                emit_error!(span, "invalid conversion method");
            }
        }
    }

    pub fn expand(&self, source_ident: &syn::Ident) -> TokenStream {
        match self {
            Self::None => quote! { value__.#source_ident },
            Self::Into => quote! { value__.#source_ident.into() },
            Self::IterInto => {
                quote! { value__.#source_ident.into_iter().map(Into::into).collect() }
            }
            Self::MapInto => quote! { value__.#source_ident.map(Into::into).collect() },
            Self::Map(func) => quote! { (#func)(value__.#source_ident) },
            Self::Skip => quote! { ::core::default::Default::default() },
        }
    }
}

pub struct FieldContext {
    new_ident: syn::Ident,
    source_ident: Option<syn::Ident>,
    conversion: ConversionMethod,
}

impl FieldContext {
    pub fn new(ident: syn::Ident) -> Self {
        Self {
            new_ident: ident,
            source_ident: None,
            conversion: ConversionMethod::None,
        }
    }

    pub fn update(&mut self, span: Span, attribute: Attr) {
        match attribute {
            Attr::Source(source) => {
                if let Some(source) = source.get_ident() {
                    if self.source_ident.replace(source.clone()).is_some() {
                        emit_error!(span, "only use one #[from(source = ...)] for a field!");
                    }
                } else {
                    emit_error!(
                        span,
                        "only use singular identifiers for #[from(source = ...)] on fields!"
                    )
                }
            }
            other => self.conversion.try_replace(other, span),
        }
    }

    pub fn expand(&self) -> TokenStream {
        let Self {
            new_ident,
            source_ident,
            conversion,
        } = self;

        let source_ident = if let Some(source_ident) = source_ident {
            source_ident
        } else {
            new_ident
        };

        let conversion = conversion.expand(source_ident);

        quote! {
            #new_ident: #conversion,
        }
    }
}

pub struct DeriveContext {
    source: syn::Path,
    new: syn::Ident,
    fields: Vec<FieldContext>,
}

impl DeriveContext {
    pub fn new(item: DeriveInput) -> Self {
        let mut source = None;
        for (span, attr) in Attr::collect(&item.attrs) {
            match attr {
                Ok(Attr::Source(src)) => {
                    if source.replace(src).is_some() {
                        emit_error!(
                            span,
                            "only one #[from(source = ...)] can be specified at the type level"
                        );
                    }
                }
                Ok(_) => emit_error!(
                    span,
                    "only #[from(source = ...)] is allowed at the type level"
                ),
                Err(e) => emit_error!(span, e),
            }
        }

        let Some(source) = source else {
            abort!(Span::call_site(), "#[from(source = ...)] must be specified at the type level");
        };

        let new_ident = item.ident.clone();
        let item_span = item.span();

        let Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) = item.data else {
            abort!(item_span, "#[derive(From)] can only be used on structs with named fields");
        };

        let fields = fields
            .named
            .into_iter()
            .map(|field| {
                let mut context = FieldContext::new(field.ident.as_ref().unwrap().clone());

                for (span, attr) in Attr::collect(&field.attrs) {
                    match attr {
                        Ok(attr) => context.update(span, attr),
                        Err(e) => emit_error!(span, e),
                    }
                }

                context
            })
            .collect();

        Self {
            source,
            new: new_ident,
            fields,
        }
    }

    pub fn expand(&self) -> TokenStream {
        let Self {
            source,
            new,
            fields,
        } = self;

        let fields = fields.iter().map(|f| f.expand());

        quote! {
            impl ::core::convert::From<#source> for #new {
                fn from(value__: #source) -> Self {
                    Self {
                        #(#fields)*
                    }
                }
            }
        }
    }
}
