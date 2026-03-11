use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Field};

pub fn extract_named_fields(input: &DeriveInput) -> Vec<&Field> {
    match &input.data {
        syn::Data::Struct(s) => match &s.fields {
            syn::Fields::Named(named) => named.named.iter().collect(),
            _ => panic!("Only named-field structs are supported"),
        },
        _ => panic!("Only structs are supported"),
    }
}

pub fn extract_iterable_trait(input: &DeriveInput) -> TokenStream {
    input
        .attrs
        .iter()
        .find(|a| a.path().is_ident("iterable"))
        .and_then(|attr| {
            if let syn::Meta::List(list) = &attr.meta {
                Some(list.tokens.clone())
            } else {
                None
            }
        })
        .unwrap_or_else(|| quote!(std::any::Any))
}

pub fn field_is_skipped(field: &syn::Field) -> bool {
    field
        .attrs
        .iter()
        .any(|attr| attr.path().is_ident("iterable") && matches!(attr.meta, syn::Meta::Path(_)))
}

pub fn expand_iterable_impl<'a>(
    struct_name: &syn::Ident,
    fields: Vec<&'a &'a syn::Field>, // Sorta messy lol
    trait_name: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let field_pairs = fields.iter().map(|field| {
        let ident = field.ident.as_ref().unwrap();
        let name = ident.to_string();
        quote! {
            (#name, &(self.#ident) as &dyn #trait_name)
        }
    });

    let fields = fields.iter().map(|field| {
        let ident = field.ident.as_ref().unwrap();
        quote! {
            &(self.#ident) as &dyn #trait_name
        }
    });

    quote! {
        impl<'a> IterableRef for &'a #struct_name {
            type Item = &'a dyn #trait_name;

            fn iter(self) -> std::vec::IntoIter<(&'static str, Self::Item)> {
                vec![#(#field_pairs),*].into_iter()
            }
            fn iter_val(self) -> std::vec::IntoIter<(Self::Item)> {
                vec![#(#fields),*].into_iter()
            }
        }
    }
}

pub fn expand_iterable_mut_impl<'a>(
    struct_name: &syn::Ident,
    fields: Vec<&'a &'a syn::Field>, // Sorta messy lol
    trait_name: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let field_pairs = fields.iter().map(|field| {
        let ident = field.ident.as_ref().unwrap();
        let name = ident.to_string();
        quote! {
            (#name, &mut (self.#ident) as &mut dyn #trait_name)
        }
    });

    let fields = fields.iter().map(|field| {
        let ident = field.ident.as_ref().unwrap();
        quote! {
            &mut (self.#ident) as &mut dyn #trait_name
        }
    });

    quote! {
        impl<'a> IterableMut for &'a mut #struct_name {
            type Item = &'a mut dyn #trait_name;

            fn iter_mut(self) -> std::vec::IntoIter<(&'static str, Self::Item)> {
                vec![#(#field_pairs),*].into_iter()
            }

            fn iter_mut_val(self) -> std::vec::IntoIter<Self::Item> {
                vec![#(#fields),*].into_iter()
            }
        }
    }
}

pub fn extract_container(ty: &syn::Type) -> ContainerType {
    if let syn::Type::Path(type_path) = ty {
        let segment = type_path.path.segments.first().unwrap();

        if segment.ident == "Option" {
            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                if let Some(syn::GenericArgument::Type(_inner)) = args.args.first() {
                    return ContainerType::Option;
                }
            }
        }

        if segment.ident == "Vec" {
            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                if let Some(syn::GenericArgument::Type(_inner)) = args.args.first() {
                    return ContainerType::Vec;
                }
            }
        }
    }

    ContainerType::Single
}
pub struct FieldInfo {
    pub name: syn::Ident,
    pub asset_ty: Option<syn::Type>,
    pub container: ContainerType,
    pub original_ty: syn::Type,
}

pub enum ContainerType {
    Single,
    Option,
    Vec,
}
pub fn analyze_asset_fields(fields: Vec<&syn::Field>) -> Vec<FieldInfo> {
    let mut result = Vec::new();

    for field in fields {
        let name = field.ident.clone().unwrap();
        let ty = field.ty.clone();

        let mut asset_ref = None;

        for attr in &field.attrs {
            if attr.path().is_ident("asset_ref") {
                asset_ref = Some(
                    attr.parse_args::<syn::Type>()
                        .expect("asset_ref requires type")
                );
            }
        }

        let container = extract_container(&ty);

        result.push(FieldInfo {
            name,
            asset_ty: asset_ref,
            container,
            original_ty: ty,
        });
    }

    result
}