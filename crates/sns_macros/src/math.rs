use proc_macro::TokenStream;
use quote::quote;

use syn::{DeriveInput, Field, Ident, Meta, Type, parse2};

pub fn gather_iterable_trait(input: &DeriveInput) -> Type {
    input
        .attrs
        .iter()
        .find(|a| a.path().is_ident("iterable"))
        .and_then(|attr| {
            if let Meta::List(list) = &attr.meta {
                // Parse the tokens as a type
                parse2::<Type>(list.tokens.clone()).ok()
            } else {
                None
            }
        })
        // Default to std::any::Any if not present
        .unwrap_or_else(|| syn::parse_str("std::any::Any").unwrap())
}

pub fn field_is_skipped(field: &Field) -> bool {
    field
        .attrs
        .iter()
        .any(|attr| attr.path().is_ident("iterable") && matches!(attr.meta, Meta::Path(_)))
}

pub fn make_field<'a>(name: &Ident, active_fields: Vec<&'a &'a Field>) -> TokenStream {
    let add_fields = active_fields.iter().map(|f| {
        let ident = f.ident.as_ref().unwrap();
        quote!(#ident: self.#ident + rhs.#ident)
    });

    let div_fields = active_fields.iter().map(|f| {
        let ident = f.ident.as_ref().unwrap();
        quote!(#ident: self.#ident / rhs.#ident)
    });

    let mul_fields = active_fields.iter().map(|f| {
        let ident = f.ident.as_ref().unwrap();
        quote!(#ident: self.#ident * rhs.#ident)
    });

    let sub_fields = active_fields.iter().map(|f| {
        let ident = f.ident.as_ref().unwrap();
        quote!(#ident: self.#ident - rhs.#ident)
    });

    let gaussian_terms = active_fields.iter().map(|f| {
        let ident = f.ident.as_ref().unwrap();
        quote! {
            sum += ((self.#ident - center.#ident) / width.#ident).powi(2);
        }
    });

    let min_fields = active_fields.iter().map(|f| {
        let ident = f.ident.as_ref().unwrap();
        quote!(#ident: self.#ident.min(rhs.#ident))
    });

    let clamp_fields = active_fields.iter().map(|f| {
        let ident = f.ident.as_ref().unwrap();
        quote!(#ident: self.#ident.clamp(-rhs.#ident,rhs.#ident))
    });

    let lerp_fields = active_fields.iter().map(|f| {
        let ident = f.ident.as_ref().unwrap();
        quote!(#ident: self.#ident + t * (self.#ident - other.#ident))
    });

    let expand = quote! {
        impl std::ops::Add for #name {
            type Output = Self;
            fn add(self, rhs: Self) -> Self {
                Self { #(#add_fields),* }
            }
        }

        impl std::ops::Sub for #name {
            type Output = Self;
            fn sub(self, rhs: Self) -> Self {
                Self { #(#sub_fields),* }
            }
        }


        impl std::ops::Mul for #name {
            type Output = Self;
            fn mul(self, rhs: Self) -> Self {
                Self { #(#mul_fields),* }
            }
        }

        impl std::ops::Div for #name {
            type Output = Self;
            fn div(self, rhs: Self) -> Self {
                Self { #(#div_fields),* }
            }
        }

        impl #name {
            pub fn gaussian(self, center: &Self, width: &Self) -> f32 {
                let mut sum = 0.0;
                #(#gaussian_terms)*
                (-sum).exp()
            }

            pub fn min(self, rhs: Self) -> Self {
                 Self { #(#min_fields),* }
            }

            pub fn clamp(self, rhs: Self) -> Self {
                 Self { #(#clamp_fields),* }
            }


            pub fn lerp(&self, other: &Self, t: f32) -> Self {
                Self { #(#lerp_fields),* }
            }
        }
    };
    TokenStream::from(expand)
}

pub fn make_iterable<'a>(
    name: &Ident,
    trait_name: Type,
    fields: Vec<&'a &'a Field>, // Sorta messy lol
) -> TokenStream {
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

    let expand = quote! {
        impl<'a> IterableRef for &'a #name {
            type Item = &'a dyn #trait_name;

            fn iter(self) -> std::vec::IntoIter<(&'static str, Self::Item)> {
                vec![#(#field_pairs),*].into_iter()
            }
            fn iter_val(self) -> std::vec::IntoIter<(Self::Item)> {
                vec![#(#fields),*].into_iter()
            }
        }
    };
    TokenStream::from(expand)
}

pub fn make_iterable_mut<'a>(
    name: &Ident,
    trait_name: Type,
    fields: Vec<&'a &'a Field>, // Sorta messy lol
) -> TokenStream {
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

    let expand = quote! {
        impl<'a> IterableMut for &'a mut #name {
            type Item = &'a mut dyn #trait_name;

            fn iter_mut(self) -> std::vec::IntoIter<(&'static str, Self::Item)> {
                vec![#(#field_pairs),*].into_iter()
            }

            fn iter_mut_val(self) -> std::vec::IntoIter<Self::Item> {
                vec![#(#fields),*].into_iter()
            }
        }
    };
    TokenStream::from(expand)
}
