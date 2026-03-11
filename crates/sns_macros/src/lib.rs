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
        quote!(#ident: lerp(self.#ident,other.#ident,t))
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

    let trait_name = utils::extract_iterable_trait(&input);
    let fields = utils::extract_named_fields(&input);
    let active_fields = fields.iter().filter(|f| !utils::field_is_skipped(f));

    utils::expand_iterable_impl(struct_name, active_fields.collect(), trait_name).into()
}

#[proc_macro_derive(AssetDef, attributes(asset_ref))]
pub fn derive_asset_def(input: TokenStream) -> TokenStream {
    // 1. Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);
    let fields = utils::extract_named_fields(&input);
    let fields_info = utils::analyze_asset_fields(fields);

    let name = &input.ident;
    let id_name = quote::format_ident!("{}ID", name);
    let raw_name = quote::format_ident!("{}Raw", name);

    let dependency_types: Vec<_> = fields_info
        .iter()
        .filter_map(|f| f.asset_ty.as_ref())
        .collect();

    let raw_fields = fields_info.iter().map(|f| {
        let name = &f.name;

        if f.asset_ty.is_some() {
            match f.container {
                utils::ContainerType::Single => quote! { pub #name: AssetRef },
                utils::ContainerType::Option => quote! { pub #name: Option<AssetRef> },
                utils::ContainerType::Vec => quote! { pub #name: Vec<AssetRef> },
            }
        } else {
            let ty = &f.original_ty;
            quote! { pub #name: #ty }
        }
    });
    let raw_id = quote! { pub id: String };

    let convert_fields = fields_info.iter().map(|f| {
        let name = &f.name;

        if let Some(asset_ty) = &f.asset_ty {
            match f.container {
                utils::ContainerType::Single => quote! {
                    let #name = registries
                        .get_mut::<#asset_ty>()
                        .expect("missing registry")
                        .intern(raw.#name);
                },

                utils::ContainerType::Option => quote! {
                    let #name = raw.#name.map(|r| {
                        registries
                            .get_mut::<#asset_ty>()
                            .expect("missing registry")
                            .intern(r)
                    });
                },

                utils::ContainerType::Vec => quote! {
                    let #name = raw.#name
                        .into_iter()
                        .map(|r| {
                            registries
                                .get_mut::<#asset_ty>()
                                .expect("missing registry")
                                .intern(r)
                        })
                        .collect();
                },
            }
        } else {
            quote! {}
        }
    });

    let final_fields = fields_info.iter().map(|f| {
        let name = &f.name;

        if f.asset_ty.is_some() {
            quote! { #name }
        } else {
            quote! { #name: raw.#name }
        }
    });

    let schema_fields = fields_info.iter().map(|f| {
        let field_ident = &f.name;

        if let Some(asset_ty) = &f.asset_ty {
            let asset_name = quote!(stringify!(#asset_ty));

            match f.container {
                utils::ContainerType::Single => quote! {
                    (stringify!(#field_ident), schema::Schema::asset(#asset_name))
                },
                utils::ContainerType::Option => quote! {
                    (stringify!(#field_ident), schema::Schema::optional_asset(#asset_name))
                },
                utils::ContainerType::Vec => quote! {
                    (stringify!(#field_ident), schema::Schema::asset_array(#asset_name))
                },
            }
        } else {
            quote! {
                (stringify!(#field_ident), schema::Schema::primitive())
            }
        }
    });

    let visit_fields = fields_info.iter().map(|f| {
        let field_ident = &f.name;

        if let Some(asset_ty) = &f.asset_ty {
            match f.container {
                utils::ContainerType::Single => 
                    quote! {
                        visitor(
                            std::any::TypeId::of::<#asset_ty>(),
                            self.#field_ident.into()
                        );
                     },
                utils::ContainerType::Option => 
                    quote! {
                        if let Some(id) = self.#field_ident {

                            visitor(
                                std::any::TypeId::of::<#asset_ty>(),
                                id.into()
                            );
                        }
                    },
                utils::ContainerType::Vec => quote! {

                    for id in &self.#field_ident {

                        visitor(
                            std::any::TypeId::of::<#asset_ty>(),
                            (*id).into()
                        );

                    }
                },
            }
        } else {
           quote! {}
        }
    });

    let expanded = quote! {
        #[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct #id_name(pub u32);

        impl From<u32> for #id_name {
            fn from(value: u32) -> Self {
                Self(value)
            }
        }

        impl Into<u32> for #id_name {
            fn into(self) -> u32 {
                self.0
            }
        }

        impl DefinedAsset for #name {
            type ID = #id_name;
            const TYPE: &'static str = stringify!(#name);
            fn create_registry() -> Box<dyn registry::AssetRegistryDyn> {
                Box::new(registry::AssetRegistry::<#id_name, #name>::default())
            }
            fn load_asset(
                registries: &mut registry::RegistrySystem,
                value: serde_json::Value
            ) {

                let raw: #raw_name = serde_json::from_value(value)
                    .expect("invalid asset json");

                let asset_ref = AssetRef::from(raw.id.clone());
                let asset = #name::from_raw(raw, registries);

                registries
                    .get_mut::<#name>()
                    .expect("registry missing")
                    .insert(asset, asset_ref);
            }
            fn dependencies() -> Vec<std::any::TypeId> {
                vec![
                    #( std::any::TypeId::of::<#dependency_types>() ),*
                ]
            }
            fn schema() -> schema::Schema {
                let mut s = schema::Schema::object();
                #( s.field(#schema_fields); )*
                s
            }
            fn visit_asset_refs<F>(&self, mut visitor: F)
            where
                F: FnMut(std::any::TypeId, u32)
            {
                #( #visit_fields )*
            }
        }
        #[derive(serde::Deserialize)]
        pub struct #raw_name {
            #raw_id,
            #( #raw_fields, )*
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

        }
    };

    TokenStream::from(expanded)
}

#[proc_macro]
pub fn register_assets(input: TokenStream) -> TokenStream {
    // 1. Parse a comma-separated list of types (e.g., Curves, Mesh, Texture)
    // Punctuated handles the optional trailing comma automatically.
    let types = parse_macro_input!(input with Punctuated::<Type, Token![,]>::parse_terminated);

    // 2. Generate the repeated calls: self.init_registry::<Type>();
    let init_calls = types.iter().map(|ty| {
        quote! {
            self.init_registry::<#ty>();
        }
    });
    let match_calls = types.iter().map(|ty| {
        quote! {
            #ty::TYPE => #ty::load_asset(self, json),
        }
    });

    let dep_calls = types.iter().map(|ty| {
        quote! {
           dependencies.insert(TypeId::of::<#ty>(),#ty::dependencies());
        }
    });

    let list_calls = types.iter().map(|ty| {
        quote! {
            types.push(#ty::TYPE);
        }
    });

    // 3. Wrap it in the impl block
    let expanded = quote! {
        impl RegistrySystem {
            pub fn init_all(&mut self) {
                #( #init_calls )*
            }
            pub fn load_by_type(
                &mut self,
                ty: &str,
                json: serde_json::Value
            ) {
                match ty {
                     #( #match_calls )*
                    _ => panic!("unknown asset type")
                }
            }
            pub fn asset_dependencies(&self) -> HashMap<TypeId, Vec<TypeId>> {
                let mut dependencies = HashMap::new();
                #( #dep_calls )*
                dependencies
            }
            pub fn get_types(&self)-> Vec<&'static str>{
                let mut types = Vec::new();
                #( #list_calls )*
                types
            }
        }
    };

    TokenStream::from(expanded)
}
