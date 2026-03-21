//Models and shared values essentially anything that mutiple parts of the project can use

use proc_macro::TokenStream;
use quote::quote;

use syn::{DeriveInput, Token, Type, parse_macro_input, punctuated::Punctuated};
mod utils;

#[proc_macro_derive(FieldMath)]
pub fn derive_field_math(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    let fields = utils::extract_named_fields(&input);
    let active_fields: Vec<&&syn::Field> = fields
        .iter()
        .filter(|f| !utils::field_is_skipped(f))
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
        quote!(#ident: self.#ident + t * (self.#ident - other.#ident))
    });

    let iterable_impl = utils::expand_iterable_impl(
        struct_name,
        active_fields.clone(),
        quote!(num_traits::ToPrimitive),
    );

    let iterable_mut_impl =
        utils::expand_iterable_mut_impl(struct_name, active_fields.clone(), quote!(std::any::Any));

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
    }
    .into()
}

#[proc_macro_derive(Iterable, attributes(iterable))]
pub fn derive_iterable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;

    let trait_name = utils::extract_iterable_trait(&input);
    let fields = utils::extract_named_fields(&input);
    let active_fields = fields.iter().filter(|f| !utils::field_is_skipped(f));

    utils::expand_iterable_impl(struct_name, active_fields.collect(), trait_name).into()
}

#[proc_macro_derive(AssetDef, attributes(asset_ref,asset))]
pub fn derive_asset_def(input: TokenStream) -> TokenStream {
    // 1. Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);
    let (type_name, namespace) = utils::parse_asset_attr(&input);
    let fields = utils::extract_named_fields(&input);
    let fields_info = utils::analyze_asset_fields(fields);

    let name = &input.ident;
    let id_name = quote::format_ident!("{}ID", name);
    let raw_name = quote::format_ident!("{}Raw", name);

    let dependency_types: Vec<_> = fields_info
        .iter()
        .filter_map(|f| f.asset_ty.as_ref())
        .collect();

    let raw_fields = fields_info.iter().map(|f| f.raw_field());
    let convert_fields = fields_info.iter().map(|f| f.convert_field());
    let final_fields = fields_info.iter().map(|f| f.final_field());
    let meta_fields = fields_info.iter().map(|f| f.meta_field());
    let visit_fields = fields_info.iter().map(|f| f.visit_field());
    let getters = fields_info.iter().map(|f| f.getter());


    let id_impl = utils::generate_id(&id_name);
    let raw_struct = utils::generate_raw(&raw_name, raw_fields);
    

    let expanded = quote! {
        #id_impl
        #raw_struct

        impl DefinedAsset for #name {
            type ID = #id_name;
            const TYPE: &'static str = #type_name;
            const NAMESPACE: &'static str = #namespace;

            fn load_asset(
                registries: &mut registry::RegistrySystem,
                value: serde_json::Value
            )->Result<(),String> {
                let raw: #raw_name = serde_json::from_value(value)
                    .map_err(|e| format!("{}: {}", #name::TYPE, e))?;
        
                let asset_ref = AssetRef::from(raw.id.clone());
                let asset = #name::from_raw(raw, registries);

                registries.insert::<#name>(asset, asset_ref);
                Ok(())

            }
            fn dependencies() -> Vec<std::any::TypeId> {
                vec![
                    #( std::any::TypeId::of::<#dependency_types>() ),*
                ]
            }
            fn current_ref() -> AssetRef {
                AssetRef {
                    name: Self::TYPE.into(),
                    namespace: Self::NAMESPACE.into()
                }
            }

            fn meta() -> metadata::Meta {
                let mut s = metadata::Meta::object();
                #( s.field(#meta_fields); )*
                s
            }
            fn visit_asset_refs<F>(&self, mut visitor: F)
            where
                F: FnMut(std::any::TypeId, u32)
            {
                #( #visit_fields )*
            }
        }
        impl #name {
            pub fn from_raw(
                raw: #raw_name,
                registries: &mut registry::RegistrySystem
            ) -> Self {

                #( #convert_fields )*
                Self {
                    #( #final_fields, )*
                }
            }
            #( #getters )*

        }
    };

    TokenStream::from(expanded)
}


#[proc_macro]
pub fn register_assets(input: TokenStream) -> TokenStream {

    let types = parse_macro_input!(input with Punctuated::<Type, Token![,]>::parse_terminated);
    
    let struct_fields = types.iter().map(|ty| {
        let field = utils::type_to_ident(ty);
        quote! {
            pub #field: AssetRegistry<<#ty as DefinedAsset>::ID, #ty>
        }
    });

    let init_fields = types.iter().map(|ty| {
        let field = utils::type_to_ident(ty);
        quote! {
            #field: AssetRegistry::default()
        }
    });
  
    let access_impls = types.iter().map(|ty| {
        let field = utils::type_to_ident(ty);
        quote! {
            impl RegistryAccess<#ty> for RegistrySystem {
                fn get(&self) -> &AssetRegistry<<#ty as DefinedAsset>::ID, #ty> {
                    &self.#field
                }

                fn get_mut(&mut self) -> &mut AssetRegistry<<#ty as DefinedAsset>::ID, #ty> {
                    &mut self.#field
                }
            }
        }
    });

    

  let loader_inserts = types.iter().map(|ty| {
        quote! {
        let loader: fn(&mut Self, &Validator, Value) -> Result<(), String> =
            |registry, validator, json| {
                match json {
                    serde_json::Value::Array(arr) => {
                        for v in arr {
                            validator.validate(&v)
                                .map_err(|e| format!("Validation failed: {}", e))?;
                            #ty::load_asset(registry, v)?;
                        }
                    }
                    v @ serde_json::Value::Object(_) => {
                        validator.validate(&v)
                            .map_err(|e| format!("Validation failed: {}", e))?;
                        #ty::load_asset(registry, v)?;
                    }
                    _ => return Err("Invalid JSON format".into())
                }
                Ok(())
            };

        loaders.insert(
            #ty::current_ref(),
            loader
        );
        }
    });

    let dep_inserts = types.iter().map(|ty| {
  
        quote!(
            let dep: fn() -> Vec<TypeId> = || {
                #ty::dependencies()
            };

            deps.insert(
                #ty::current_ref(),
                dep
            );
        )
    });
    let meta_inserts = types.iter().map(|ty| {
        quote!(
            let meta: fn() -> metadata::Meta = || {
                #ty::meta()
            };

            metadata.insert(
            #ty::current_ref(),
            meta
        );
        )
    });

    let get_type = types.iter().map(|ty| {
        quote!(
            #ty::TYPE,
        )
    });
    let get_refs = types.iter().map(|ty| {
        quote!(
            #ty::current_ref(),
        )
    });


    let expanded = quote! {

        trait RegistryAccess<T: DefinedAsset> {
            fn get(&self) -> &AssetRegistry<T::ID, T>;
            fn get_mut(&mut self) -> &mut AssetRegistry<T::ID, T>;
        }

        pub struct RegistrySystem  {
            loaders: HashMap<AssetRef, fn(&mut Self, &Validator, Value) -> Result<(), String>>,
            metadata: HashMap<AssetRef, fn() -> metadata::Meta>,
            deps: HashMap<AssetRef, fn() -> Vec<TypeId>>,
            #( #struct_fields, )*
        }
        impl Default for RegistrySystem {
            fn default() -> Self {
                let mut loaders = HashMap::new();
                let mut metadata = HashMap::new();
                let mut deps = HashMap::new();

                #( #loader_inserts )*
                #( #meta_inserts )*
                #( #dep_inserts )*
                

                Self {
                    loaders,
                    metadata,
                    deps,
                    #( #init_fields, )*
                }
            }
        }

        impl RegistrySystem {
           pub fn load_by_type(
                &mut self,
                r: &AssetRef,
                validator: &Validator,
                json: Value
            ) -> Result<(), String> {
                let loader = self.loaders
                .get(r)
                .ok_or_else(|| format!("unknown asset type '{}'", r))?;

                loader(self, validator, json)
            }
            pub fn get_meta(&self, r: &AssetRef) -> Option<metadata::Meta> {
                self.metadata.get(r).map(|f| f())
            }
            pub fn dependencies_of(&self, r: &AssetRef) -> Option<Vec<TypeId>> {
                self.deps.get(r).map(|f| f())
            }
            pub fn get_types(&self)-> Vec<&'static str>{
                vec![
                    #( #get_type )*
                ]
            }
            pub fn get_refs(&self) -> Vec<AssetRef> {
                vec![
                    #( #get_refs )*
                ]
            }

            pub(crate) fn get<T: DefinedAsset>(&self) -> &AssetRegistry<T::ID, T>
            where
                Self: RegistryAccess<T>,
            {
                <Self as RegistryAccess<T>>::get(self)
            }

            pub(crate) fn get_mut<T: DefinedAsset>(&mut self) -> &mut AssetRegistry<T::ID, T>
            where
                Self: RegistryAccess<T>,
            {
                <Self as RegistryAccess<T>>::get_mut(self)
            }

            pub(crate) fn intern<T: DefinedAsset>(&mut self, r: AssetRef) -> T::ID
            where
                Self: RegistryAccess<T>,
            {
                self.get_mut::<T>().intern(r)
            }

            pub(crate) fn insert<T: DefinedAsset>(&mut self, asset: T, r: AssetRef) -> T::ID
            where
                Self: RegistryAccess<T>,
            {
                self.get_mut::<T>().insert(asset, r)
            }

        }
       
        #(#access_impls)*

    };

    TokenStream::from(expanded)
}
