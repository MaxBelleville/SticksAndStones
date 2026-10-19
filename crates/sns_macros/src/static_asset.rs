use proc_macro::TokenStream;
use quote::quote;

use super::utils;
use syn::{Ident, Type, punctuated::Punctuated, token::Comma};

pub fn make(
    name: &Ident,
    id_name: Ident,
    raw_name: Ident,
    type_name: String,
    namespace: String,
    fields_info: &Vec<utils::FieldInfo>,
) -> TokenStream {
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

        impl StaticAsset for #name {
            type ID = #id_name;
            const TYPE: &'static str = #type_name;
            const NAMESPACE: &'static str = #namespace;

            fn load_asset(
                registries: &mut registry::RegistrySystem,
                defs: &registry::DefinedRegistrySystem,
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

pub fn register(types: Punctuated<Type, Comma>) -> TokenStream {
    let struct_fields = types.iter().map(|ty| {
        let field = utils::type_to_ident(ty);
        quote! {
            pub #field: AssetRegistry<<#ty as StaticAsset>::ID, #ty>
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
                fn get(&self) -> &AssetRegistry<<#ty as StaticAsset>::ID, #ty> {
                    &self.#field
                }

                fn get_mut(&mut self) -> &mut AssetRegistry<<#ty as StaticAsset>::ID, #ty> {
                    &mut self.#field
                }
            }
        }
    });

    let loader_inserts = types.iter().map(|ty| {
        quote! {
        let loader: fn(&mut Self,  &DefinedRegistrySystem, &Validator, Value) -> Result<(), String> =
            |registry, defs, validator, json| {
                match json {
                    serde_json::Value::Array(arr) => {
                        for v in arr {
                            validator.validate(&v)
                                .map_err(|e| format!("Validation failed: {}", e))?;
                            #ty::load_asset(registry,defs, v)?;
                        }
                    }
                    v @ serde_json::Value::Object(_) => {
                        validator.validate(&v)
                            .map_err(|e| format!("Validation failed: {}", e))?;
                        #ty::load_asset(registry, defs, v)?;
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

        trait RegistryAccess<T: StaticAsset> {
            fn get(&self) -> &AssetRegistry<T::ID, T>;
            fn get_mut(&mut self) -> &mut AssetRegistry<T::ID, T>;
        }

        pub struct RegistrySystem  {
            loaders: HashMap<AssetRef, fn(&mut Self, &DefinedRegistrySystem, &Validator, Value) -> Result<(), String>>,
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
                defs: &DefinedRegistrySystem,
                validator: &Validator,
                json: Value
            ) -> Result<(), String> {
                let loader = self.loaders
                .get(r)
                .ok_or_else(|| format!("unknown asset type '{}'", r))?;

                loader(self, defs, validator, json)
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

            pub(crate) fn get<T: StaticAsset>(&self) -> &AssetRegistry<T::ID, T>
            where
                Self: RegistryAccess<T>,
            {
                <Self as RegistryAccess<T>>::get(self)
            }

            pub(crate) fn get_mut<T: StaticAsset>(&mut self) -> &mut AssetRegistry<T::ID, T>
            where
                Self: RegistryAccess<T>,
            {
                <Self as RegistryAccess<T>>::get_mut(self)
            }

            pub(crate) fn intern<T: StaticAsset>(&mut self, r: AssetRef) -> T::ID
            where
                Self: RegistryAccess<T>,
            {
                self.get_mut::<T>().intern(r)
            }

            pub(crate) fn insert<T: StaticAsset>(&mut self, asset: T, r: AssetRef) -> T::ID
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
