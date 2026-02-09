
use syn::{DeriveInput, Field};
use proc_macro2::TokenStream;
use quote::quote;

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
    field.attrs.iter().any(|attr| {
        attr.path().is_ident("iterable")
            && matches!(attr.meta, syn::Meta::Path(_))
    })
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