use proc_macro::TokenStream;

mod attr;
mod context;
mod meta;

#[proc_macro_derive(From, attributes(from))]
pub fn derive_from(item: TokenStream) -> TokenStream {
    let item = syn::parse_macro_input!(item as syn::DeriveInput);
    let context = context::DeriveContext::new(item);

    TokenStream::from(context.expand())
}
