use proc_macro::TokenStream;
use quote::quote;

use super::utils;
use syn::{punctuated::Punctuated, token::Comma, Ident, Type};

pub fn make(
    name: &Ident,
    id_name: Ident,
    raw_name: Ident,
    type_name: String,
    namespace: String,
    fields_info: &Vec<utils::FieldInfo>,
) -> TokenStream {
    let raw_fields = fields_info.iter().map(|f| f.raw_field());
    let convert_fields = fields_info.iter().map(|f| f.convert_field());
    let final_fields = fields_info.iter().map(|f| f.final_field());
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

          fn load_definition(
              registries: &mut registry::DefinedRegistrySystem,
              value: serde_json::Value
          )->Result<(),String> {
                  let raw: #raw_name = serde_json::from_value(value)
                      .map_err(|e| format!("{}: {}", #name::TYPE, e))?;

                  let asset_ref = AssetRef::from(raw.id.clone());
                  let asset = #name::from_raw(raw, registries);

                  registries.insert::<#name>(asset, asset_ref);
                  Ok(())
          }

          fn current_ref() -> AssetRef {
              AssetRef {
                  name: Self::TYPE.into(),
                  namespace: Self::NAMESPACE.into()
              }
          }

      }
      impl #name {
          pub fn from_raw(
              raw: #raw_name,
              registries: &mut registry::DefinedRegistrySystem
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
    let init_fields = types.iter().map(|ty| {
        let field = utils::type_to_ident(ty);
        quote! {
            #field: AssetRegistry::default()
        }
    });

    let struct_fields = types.iter().map(|ty| {
        let field = utils::type_to_ident(ty);
        quote! {
            pub #field: AssetRegistry<<#ty as DefinedAsset>::ID, #ty>
        }
    });

    let access_impls = types.iter().map(|ty| {
        let field = utils::type_to_ident(ty);
        quote! {
            impl DefinedRegistryAccess<#ty> for DefinedRegistrySystem {
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
                            #ty::load_definition(registry, v)?;
                        }
                    }
                    v @ serde_json::Value::Object(_) => {
                        validator.validate(&v)
                            .map_err(|e| format!("Validation failed: {}", e))?;
                        #ty::load_definition(registry, v)?;
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
                trait DefinedRegistryAccess<T: DefinedAsset> {
                    fn get(&self) -> &AssetRegistry<T::ID, T>;
                    fn get_mut(&mut self) -> &mut AssetRegistry<T::ID, T>;
                }
         pub struct DefinedRegistrySystem  {
             loaders: HashMap<AssetRef, fn(&mut Self, &Validator, Value) -> Result<(), String>>,
             #(#struct_fields),*
         }

         impl Default for DefinedRegistrySystem {
             fn default() -> Self {
                 let mut loaders = HashMap::new();

                 #( #loader_inserts )*


                 Self {
                     loaders,
                     #( #init_fields, )*
                 }
             }
         }

        impl DefinedRegistrySystem {
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
                 Self: DefinedRegistryAccess<T>,
             {
                 <Self as DefinedRegistryAccess<T>>::get(self)
             }

             pub(crate) fn get_mut<T: DefinedAsset>(&mut self) -> &mut AssetRegistry<T::ID, T>
             where
                 Self: DefinedRegistryAccess<T>,
             {
                 <Self as DefinedRegistryAccess<T>>::get_mut(self)
             }

             pub(crate) fn intern<T: DefinedAsset>(&mut self, r: AssetRef) -> T::ID
             where
                 Self: DefinedRegistryAccess<T>,
             {
                 self.get_mut::<T>().intern(r)
             }

             pub(crate) fn insert<T: DefinedAsset>(&mut self, asset: T, r: AssetRef) -> T::ID
             where
                 Self: DefinedRegistryAccess<T>,
             {
                 self.get_mut::<T>().insert(asset, r)
             }

         }

         #(#access_impls)*

    };
    TokenStream::from(expanded)
}
