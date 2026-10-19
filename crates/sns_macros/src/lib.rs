//Models and shared values essentially anything that mutiple parts of the project can use

use proc_macro::TokenStream;

use syn::{DeriveInput, Token, Type, parse_macro_input, punctuated::Punctuated};
mod defined_asset;
mod math;
mod static_asset;
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

#[proc_macro_derive(MakeAsset, attributes(asset_ref, asset, enum_ref))]
pub fn derive_make_asset(input: TokenStream) -> TokenStream {
    // 1. Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);
    let (type_name, namespace) = utils::parse_asset_attr(&input);
    let fields = utils::extract_named_fields(&input);
    let fields_info = utils::analyze_asset_fields(fields);

    let name = &input.ident;
    let id_name = quote::format_ident!("{}ID", name);
    let raw_name = quote::format_ident!("{}Raw", name);

    static_asset::make(name, id_name, raw_name, type_name, namespace, &fields_info)
}

#[proc_macro_derive(MakeDefinedAsset, attributes(asset))]
pub fn derive_make_defined_asset(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let (type_name, namespace) = utils::parse_asset_attr(&input);
    let fields = utils::extract_named_fields(&input);
    let fields_info = utils::analyze_asset_fields(fields);

    let name = &input.ident;
    let id_name = quote::format_ident!("{}ID", name);
    let raw_name = quote::format_ident!("{}Raw", name);

    defined_asset::make(name, id_name, raw_name, type_name, namespace, &fields_info)
}

#[proc_macro]
pub fn register_assets(input: TokenStream) -> TokenStream {
    let types = parse_macro_input!(input with Punctuated::<Type, Token![,]>::parse_terminated);
    static_asset::register(types)
}

#[proc_macro]
pub fn register_defined_assets(input: TokenStream) -> TokenStream {
    let types = parse_macro_input!(input with Punctuated::<Type, Token![,]>::parse_terminated);
    defined_asset::register(types)
}
