//Models and shared values essentially anything that mutiple parts of the project can use

use proc_macro::TokenStream;

use syn::{DeriveInput, Token, Type, parse_macro_input, punctuated::Punctuated};
mod math;
mod utils;

#[proc_macro_derive(FieldMath)]
pub fn derive_make_field_math(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let fields = utils::extract_named_fields(&input);
    let active_fields = fields.iter().filter(|f| !math::field_is_skipped(f));

    math::make_field(name, active_fields.collect())
}

#[proc_macro_derive(Iterable, attributes(iterable))]
pub fn derive_make_iterable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let trait_name = math::gather_iterable_trait(&input);
    let fields = utils::extract_named_fields(&input);
    let active_fields = fields.iter().filter(|f| !math::field_is_skipped(f));

    math::make_iterable(name, trait_name, active_fields.collect())
}

#[proc_macro_derive(IterableMut, attributes(iterable))]
pub fn derive_make_iterable_mut(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let trait_name = math::gather_iterable_trait(&input);
    let fields = utils::extract_named_fields(&input);
    let active_fields = fields.iter().filter(|f| !math::field_is_skipped(f));

    math::make_iterable_mut(name, trait_name, active_fields.collect())
}
