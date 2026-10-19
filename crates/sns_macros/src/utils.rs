use heck::ToSnakeCase;
use proc_macro2::TokenStream;
use quote::quote;

pub fn extract_named_fields(input: &syn::DeriveInput) -> Vec<&syn::Field> {
    match &input.data {
        syn::Data::Struct(s) => match &s.fields {
            syn::Fields::Named(named) => named.named.iter().collect(),
            _ => panic!("Only named-field structs are supported"),
        },
        _ => panic!("Only structs are supported"),
    }
}

pub fn is_copy_type(ty: &syn::Type) -> bool {
    match ty {
        syn::Type::Path(type_path) => {
            let ident = &type_path.path.segments.last().unwrap().ident;

            matches!(
                ident.to_string().as_str(),
                // primitives
                "u8" | "u16" | "u32" | "u64" |
                "i8" | "i16" | "i32" | "i64" |
                "f32" | "f64" | "Vec3" | "Vec4" | "Vec2" |
                "bool" | "char" |

                // common small types
                "usize" | "isize"
            )
        }

        // tuples like (f32, f32)
        syn::Type::Tuple(tuple) => tuple.elems.iter().all(is_copy_type),

        // references are Copy
        syn::Type::Reference(_) => true,

        _ => false,
    }
}

pub fn type_to_ident(ty: &syn::Type) -> syn::Ident {
    if let syn::Type::Path(type_path) = ty {
        let ident = &type_path.path.segments.last().unwrap().ident;
        let name = ident.to_string().to_snake_case();
        quote::format_ident!("{}", name)
    } else {
        panic!("Unsupported type in register_assets!");
    }
}

// INFO: --- Asset Macro Helpers.

pub struct FieldInfo {
    pub name: syn::Ident,
    pub ty: syn::Type,
    pub asset_ty: Option<syn::Type>,
    pub enum_ref: Option<String>,
    pub container: ContainerType,
}
impl FieldInfo {
    pub fn raw_field(&self) -> TokenStream {
        let name = &self.name;
        let ty = &self.ty;
        if self.asset_ty.is_some() || self.enum_ref.is_some() {
            return match self.container {
                ContainerType::Single => quote! { pub #name: AssetRef },
                ContainerType::Option => quote! { pub #name: Option<AssetRef> },
                ContainerType::Vec => quote! { pub #name: Vec<AssetRef> },
            };
        }

        quote! { pub #name: #ty }
    }
    pub fn convert_field(&self) -> TokenStream {
        let name = &self.name;

        if let Some(asset_ty) = &self.asset_ty {
            return match self.container {
                ContainerType::Single => quote! {
                    let #name = registries.intern::<#asset_ty>(raw.#name);
                },

                ContainerType::Option => quote! {
                    let #name = raw.#name.map(|r| {
                       registries.intern::<#asset_ty>(r)
                    });
                },

                ContainerType::Vec => quote! {
                    let #name = raw.#name
                        .into_iter()
                        .map(|r| {
                           registries.intern::<#asset_ty>(r)
                        })
                        .collect();
                },
            };
        }

        if let Some(enum_ref) = &self.enum_ref {
            return match self.container {
                ContainerType::Single => {
                    let enum_str = enum_ref.as_str();
                    quote! {
                        let #name = AssetRef::from(#enum_str.to_string());
                    }
                }
                ContainerType::Option => {
                    let enum_str = enum_ref.as_str();
                    quote! {
                        let #name = raw.#name.as_ref().map(|_| AssetRef::from(#enum_str.to_string()));
                    }
                }
                ContainerType::Vec => {
                    let enum_str = enum_ref.as_str();
                    quote! {
                        let #name = raw.#name
                            .iter()
                            .map(|_| AssetRef::from(#enum_str.to_string()))
                            .collect();
                    }
                }
            };
        }

        quote! {}
    }
    pub fn final_field(&self) -> TokenStream {
        let name = &self.name;

        if self.asset_ty.is_some() || self.enum_ref.is_some() {
            return quote! { #name };
        }
        quote! { #name: raw.#name }
    }
    pub fn meta_field(&self) -> TokenStream {
        let field_ident = &self.name;

        if let Some(asset_ty) = &self.asset_ty {
            let asset_name = quote!(stringify!(#asset_ty));

            return match self.container {
                ContainerType::Single => quote! {
                    (stringify!(#field_ident), metadata::Meta::asset(#asset_name))
                },
                ContainerType::Option => quote! {
                    (stringify!(#field_ident), metadata::Meta::optional_asset(#asset_name))
                },
                ContainerType::Vec => quote! {
                    (stringify!(#field_ident), metadata::Meta::asset_array(#asset_name))
                },
            };
        }

        if let Some(enum_ref) = &self.enum_ref {
            let enum_str = enum_ref.as_str();
            return match self.container {
                ContainerType::Single => quote! {
                    (stringify!(#field_ident), metadata::Meta::asset(#enum_str))
                },
                ContainerType::Option => quote! {
                    (stringify!(#field_ident), metadata::Meta::optional_asset(#enum_str))
                },
                ContainerType::Vec => quote! {
                    (stringify!(#field_ident), metadata::Meta::asset_array(#enum_str))
                },
            };
        }

        quote! {
            (stringify!(#field_ident), metadata::Meta::primitive())
        }
    }
    pub fn visit_field(&self) -> TokenStream {
        let field_ident = &self.name;

        if let Some(asset_ty) = &self.asset_ty {
            return match self.container {
                ContainerType::Single => quote! {
                   visitor(
                       std::any::TypeId::of::<#asset_ty>(),
                       self.#field_ident.into()
                   );
                },
                ContainerType::Option => quote! {
                    if let Some(id) = self.#field_ident {

                        visitor(
                            std::any::TypeId::of::<#asset_ty>(),
                            id.into()
                        );
                    }
                },
                ContainerType::Vec => quote! {

                    for id in &self.#field_ident {

                        visitor(
                            std::any::TypeId::of::<#asset_ty>(),
                            (*id).into()
                        );

                    }
                },
            };
        }
        quote! {}
    }
    pub fn getter(&self) -> TokenStream {
        let name = &self.name;
        let ty = &self.ty;

        if is_copy_type(&self.ty) {
            return quote! {
                pub fn #name(&self) -> #ty {
                    self.#name
                }
            };
        }
        quote! {
            pub fn #name(&self) -> &#ty {
                &self.#name
            }
        }
    }
}

pub enum ContainerType {
    Single,
    Option,
    Vec,
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

pub fn analyze_asset_fields(fields: Vec<&syn::Field>) -> Vec<FieldInfo> {
    let mut result = Vec::new();

    for field in fields {
        let name = field.ident.clone().unwrap();
        let ty = field.ty.clone();

        let mut asset_ref = None;
        let mut enum_ref = None;

        for attr in &field.attrs {
            if attr.path().is_ident("asset_ref") {
                asset_ref = Some(
                    attr.parse_args::<syn::Type>()
                        .expect("asset_ref requires type"),
                );
            } else if attr.path().is_ident("enum_ref") {
                enum_ref = Some(
                    attr.parse_args::<syn::LitStr>()
                        .expect("enum_ref requires a string literal")
                        .value(),
                );
            }
        }

        let container = extract_container(&ty);

        result.push(FieldInfo {
            name,
            ty: ty,
            asset_ty: asset_ref,
            enum_ref,
            container,
        });
    }

    result
}

pub fn parse_asset_attr(input: &syn::DeriveInput) -> (String, String) {
    let mut ty = None;
    let mut namespace = None;

    for attr in &input.attrs {
        if attr.path().is_ident("asset") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("type") {
                    ty = Some(meta.value()?.parse::<syn::LitStr>()?.value());
                } else if meta.path.is_ident("namespace") {
                    namespace = Some(meta.value()?.parse::<syn::LitStr>()?.value());
                }
                Ok(())
            })
            .unwrap();
        }
    }

    (
        ty.unwrap_or_else(|| input.ident.to_string().to_snake_case()),
        namespace.unwrap_or_else(|| "default".to_string()),
    )
}

pub fn generate_id(id_name: &syn::Ident) -> TokenStream {
    quote! {
        #[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
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
    }
}

pub fn generate_raw(
    raw_name: &syn::Ident,
    raw_fields: impl Iterator<Item = TokenStream>,
) -> TokenStream {
    let raw_id = quote! { pub id: String };

    quote! {
        #[derive(serde::Deserialize)]
        struct #raw_name {
            #raw_id,
            #( #raw_fields, )*
        }
    }
}
