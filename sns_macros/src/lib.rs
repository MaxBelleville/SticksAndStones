//Models and shared values essentially anything that mutiple parts of the project can use

use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};
mod iterable_utils;

#[proc_macro_derive(FieldMath)]
pub fn derive_field_math(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    let fields = iterable_utils::extract_named_fields(&input);
    let active_fields: Vec<&&syn::Field> = fields
        .iter()
        .filter(|f| !iterable_utils::field_is_skipped(f))
        .collect();

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
        quote!(#ident: lerp(self.#ident,other.#ident,t))
    });

    let iterable_impl = iterable_utils::expand_iterable_impl(
        struct_name,
        active_fields.clone(),
        quote!(num_traits::ToPrimitive),
    );

    let iterable_mut_impl = iterable_utils::expand_iterable_mut_impl(
        struct_name,
        active_fields.clone(),
        quote!(std::any::Any),
    );

    quote! {
        #iterable_impl

        #iterable_mut_impl

        impl std::ops::Add for #struct_name {
            type Output = Self;
            fn add(self, rhs: Self) -> Self {
                Self { #(#add_fields),* }
            }
        }

        impl std::ops::Sub for #struct_name {
            type Output = Self;
            fn sub(self, rhs: Self) -> Self {
                Self { #(#sub_fields),* }
            }
        }


        impl std::ops::Mul for #struct_name {
            type Output = Self;
            fn mul(self, rhs: Self) -> Self {
                Self { #(#mul_fields),* }
            }
        }

        impl std::ops::Div for #struct_name {
            type Output = Self;
            fn div(self, rhs: Self) -> Self {
                Self { #(#div_fields),* }
            }
        }

        impl #struct_name {
            pub fn gaussian(self, center: Self, width: Self) -> f32 {
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
    }
    .into()
}

#[proc_macro_derive(Iterable, attributes(iterable))]
pub fn derive_iterable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;

    let trait_name = iterable_utils::extract_iterable_trait(&input);
    let fields = iterable_utils::extract_named_fields(&input);
    let active_fields = fields
        .iter()
        .filter(|f| !iterable_utils::field_is_skipped(f));

    iterable_utils::expand_iterable_impl(struct_name, active_fields.collect(), trait_name).into()
}
