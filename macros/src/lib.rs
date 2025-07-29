#![allow(clippy::uninlined_format_args)]

use std::collections::HashSet;

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::{ToTokens, quote, quote_spanned};
use syn::{
    Attribute, Data, DataEnum, DataStruct, DeriveInput, Expr, ExprLit, Field, Fields, FieldsNamed,
    FieldsUnnamed, Lit, LitInt, Member, TypePath, parse_macro_input, parse_quote, spanned::Spanned,
};

fn error(msg: String, span: Span) -> TokenStream {
    quote_spanned! {span=> compile_error!(#msg);}.into()
}

fn struct_parse_fields(data_struct: &DataStruct) -> impl IntoIterator<Item = (Field, Member)> {
    let fields = &data_struct.fields;
    fields.clone().into_iter().zip(fields.members())
}

fn get_attr(input: &DeriveInput, attr_name: &str) -> Option<Attribute> {
    input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident(attr_name))
        .cloned()
}

fn assert_trait(
    span: Span,
    ty: proc_macro2::TokenStream,
    trait_: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let type_name = format!("_AssertTrait{}", rand::random::<u64>());
    let type_name = Ident::new(&type_name, span);
    quote_spanned! {span=>
        struct #type_name where #ty: #trait_;
    }
}

#[proc_macro_derive(Serialize, attributes(sb_id, enum_repr))]
pub fn serialize_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match input.data {
        Data::Struct(ref s) => get_from_result(serialize_derive_struct(&input, s)),
        Data::Enum(ref e) => get_from_result(serialize_derive_enum(&input, e)),
        _ => error("Union not supported".to_string(), input.span()),
    }
}

fn serialize_derive_enum(
    input: &DeriveInput,
    data_enum: &DataEnum,
) -> Result<TokenStream, TokenStream> {
    commond_checks_enum(input, data_enum)?;
    let span = input.span();
    let name = &input.ident;
    let generics = &input.generics;
    let mut generics_with_params = generics.clone();
    for param in generics_with_params.type_params_mut() {
        param.bounds.push(parse_quote!(crate::data::Serialize));
    }
    let generics_types: HashSet<_> = generics
        .type_params()
        .map(|param| param.ident.clone())
        .collect();

    let repr: Ident = if let Some(attr) = get_attr(input, "enum_repr") {
        let list = attr
            .meta
            .require_list()
            .map_err(syn::Error::into_compile_error)?;
        list.parse_args().map_err(syn::Error::into_compile_error)?
    } else {
        return Err(error("Missing enum_repr attribute".to_string(), span));
    };

    let mut asserts = Vec::new();

    let (mut size_lines, mut serialize_lines) = (Vec::new(), Vec::new());
    let mut current_discriminant = 0;

    for variant in &data_enum.variants {
        let discriminant = Lit::Int(LitInt::new(&current_discriminant.to_string(), span));
        let create_repr =
            quote! {<#repr as crate::utils::macros::EnumRepr>::from_value(#discriminant)};

        if let Some((
            _,
            Expr::Lit(ExprLit {
                lit: Lit::Int(ref val),
                ..
            }),
        )) = variant.discriminant
        {
            let val: usize = val.base10_parse().map_err(syn::Error::into_compile_error)?;
            current_discriminant = val;
        }

        let span = variant.span();
        let name = &variant.ident;

        for field in &variant.fields {
            if let syn::Type::Path(TypePath { path, .. }) = &field.ty
                && let Some(ident) = path.get_ident()
                && generics_types.contains(ident)
            {
                continue;
            }

            asserts.push(assert_trait(
                span,
                field.ty.to_token_stream(),
                quote! {crate::data::Serialize},
            ));
        }

        let (idents, apply_delimiters): (_, &dyn Fn(proc_macro2::TokenStream) -> _) =
            match &variant.fields {
                Fields::Unit => (Vec::new(), &|i| i),
                Fields::Named(FieldsNamed { named, .. }) => (
                    named
                        .iter()
                        .map(|field| field.ident.clone().expect("Fields are named"))
                        .collect(),
                    &|inner| quote! {{#inner}},
                ),
                Fields::Unnamed(FieldsUnnamed { unnamed, .. }) => {
                    let len = unnamed.len();
                    (
                        (0..len)
                            .map(|i| Ident::new(&format!("_{}", i), unnamed.span()))
                            .collect(),
                        &|inner| quote! {(#inner)},
                    )
                }
            };

        let with_delimiters = apply_delimiters(quote! {#(#idents,)*});
        let match_pattern = quote! {Self::#name #with_delimiters };

        let size = quote! {#match_pattern => #create_repr.size() #(+ #idents.size())*};
        let serialize = quote! {#match_pattern => {#create_repr.serialize(stream)?; #(#idents.serialize(stream)?;)* Ok(())}};

        size_lines.push(size);
        serialize_lines.push(serialize);

        current_discriminant += 1;
    }

    Ok((quote_spanned! {span=>
        #(#asserts)*

        #[allow(clippy::just_underscores_and_digits)]
        impl #generics_with_params crate::data::Serialize for #name #generics {
            fn size(&self) -> usize {
                match self {
                    #(#size_lines),*
                }
            }


            fn serialize(&self, stream: &mut crate::data::DataStream) -> Result<(), crate::data::SerializeError> {
                match self {
                    #(#serialize_lines),*
                }
            }
        }
    })
    .into())
}

fn serialize_derive_struct(
    input: &DeriveInput,
    data_struct: &DataStruct,
) -> Result<TokenStream, TokenStream> {
    common_checks_struct(input, data_struct)?;

    let span = input.span();
    let ident = input.ident.clone();

    let generics = &input.generics;
    let mut generics_with_params = generics.clone();
    for param in generics_with_params.type_params_mut() {
        param.bounds.push(parse_quote!(crate::data::Serialize));
    }
    let generics_types: HashSet<_> = generics
        .type_params()
        .map(|param| param.ident.clone())
        .collect();

    let id_code = if let Some(attr) = get_attr(input, "sb_id") {
        let sb_id = &attr
            .meta
            .require_name_value()
            .map_err(syn::Error::into_compile_error)?
            .value;
        quote_spanned! {span=>
            impl crate::packets::ServerboundPacket for #ident {
                const ID: u32 = #sb_id;
            }

        }
    } else {
        quote! {}
    };

    let mut asserts = Vec::new();
    let fields = struct_parse_fields(data_struct);

    let (field_codes_size, field_codes_serialize): (Vec<_>, Vec<_>) = fields
        .into_iter()
        .map(|(field, member)| {
            let span = member.span();

            let is_generic = if let syn::Type::Path(TypePath { path, .. }) = &field.ty
                && let Some(ident) = path.get_ident()
            {
                generics_types.contains(ident)
            } else {
                false
            };

            if !is_generic {
                asserts.push(assert_trait(
                    span,
                    field.ty.to_token_stream(),
                    quote_spanned! {span=> crate::data::Serialize},
                ));
            }

            let size = quote_spanned! {span=> n += self.#member.size();};
            let serialize = quote_spanned! {span=> self.#member.serialize(stream)?;};

            (size, serialize)
        })
        .unzip();

    let ident = &input.ident;

    Ok(quote_spanned! {span=>
        #(#asserts)*

        #id_code

        impl #generics_with_params crate::data::Serialize for #ident #generics {
            #[allow(unused_mut, clippy::let_and_return)]
            fn size(&self) -> usize {
                let mut n = 0;
                #(#field_codes_size)*
                n
            }

            #[allow(unused_variables)]
            fn serialize(&self, stream: &mut crate::data::DataStream) -> Result<(), crate::data::SerializeError> {
                #(#field_codes_serialize)*
                Ok(())
            }
        }
    }.into())
}

#[proc_macro_derive(Deserialize, attributes(cb_id, enum_repr))]
pub fn deserialize_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match input.data {
        Data::Struct(ref s) => get_from_result(deserialize_derive_struct(&input, s)),
        Data::Enum(ref e) => get_from_result(deserialize_derive_enum(&input, e)),
        _ => error("Union not supported".to_string(), input.span()),
    }
}

fn deserialize_derive_enum(
    input: &DeriveInput,
    data_enum: &DataEnum,
) -> Result<TokenStream, TokenStream> {
    commond_checks_enum(input, data_enum)?;
    let span = input.span();
    let name = &input.ident;
    let generics = &input.generics;
    let mut generics_with_params = generics.clone();
    for param in generics_with_params.type_params_mut() {
        param.bounds.push(parse_quote!(crate::data::Deserialize));
    }
    let generics_types: HashSet<_> = generics
        .type_params()
        .map(|param| param.ident.clone())
        .collect();

    let repr: Ident = if let Some(attr) = get_attr(input, "enum_repr") {
        let list = attr
            .meta
            .require_list()
            .map_err(syn::Error::into_compile_error)?;
        list.parse_args().map_err(syn::Error::into_compile_error)?
    } else {
        return Err(error("Missing enum_repr attribute".to_string(), span));
    };

    let mut asserts = Vec::new();

    let mut deserialize_lines = Vec::new();
    let mut current_discriminant = 0;

    for variant in &data_enum.variants {
        let discriminant = Lit::Int(LitInt::new(&current_discriminant.to_string(), span));

        if let Some((
            _,
            Expr::Lit(ExprLit {
                lit: Lit::Int(ref val),
                ..
            }),
        )) = variant.discriminant
        {
            let val: usize = val.base10_parse().map_err(syn::Error::into_compile_error)?;
            current_discriminant = val;
        }

        let span = variant.span();
        let name = &variant.ident;

        for field in &variant.fields {
            if let syn::Type::Path(TypePath { path, .. }) = &field.ty
                && let Some(ident) = path.get_ident()
                && generics_types.contains(ident)
            {
                continue;
            }
            asserts.push(assert_trait(
                span,
                field.ty.to_token_stream(),
                quote! {crate::data::Deserialize},
            ));
        }

        let deserialize = match &variant.fields {
            Fields::Unit => quote! {#discriminant => Ok(Self::#name)},
            Fields::Named(FieldsNamed { named, .. }) => {
                let (idents, types): (Vec<_>, Vec<_>) = named
                    .iter()
                    .map(|field| (field.ident.as_ref().expect("Fields are named"), &field.ty))
                    .unzip();
                quote! {#discriminant => Ok(Self::#name { #(#idents: <#types>::deserialize(stream)?),* })}
            }
            Fields::Unnamed(FieldsUnnamed { unnamed, .. }) => {
                let types = unnamed.iter().map(|field| &field.ty);
                quote! {#discriminant => Ok(Self::#name ( #(<#types>::deserialize(stream)?),* ))}
            }
        };
        deserialize_lines.push(deserialize);

        current_discriminant += 1;
    }

    Ok((quote_spanned! {span=>
        // #(#asserts)*

        impl #generics_with_params crate::data::Deserialize for #name #generics {
            fn deserialize(stream: &mut crate::data::DataStream) -> Result<Self, crate::data::DeserializeError> {
                let repr = <#repr>::deserialize(stream)?;
                match crate::utils::macros::EnumRepr::to_value(repr) {
                    #(#deserialize_lines,)*
                    other => Err(crate::data::DeserializeError::MalformedPacket(format!("{}: invalid type {}", stringify!(#name), other)))
                }
            }
        }
    })
    .into())
}

fn deserialize_derive_struct(
    input: &DeriveInput,
    data_struct: &DataStruct,
) -> Result<TokenStream, TokenStream> {
    common_checks_struct(input, data_struct)?;

    let span = input.span();
    let ident = input.ident.clone();

    let generics = &input.generics;
    let mut generics_with_params = generics.clone();
    for param in generics_with_params.type_params_mut() {
        param.bounds.push(parse_quote!(crate::data::Deserialize));
    }
    let generics_types: HashSet<_> = generics
        .type_params()
        .map(|param| param.ident.clone())
        .collect();

    let id_code = if let Some(attr) = get_attr(input, "cb_id") {
        let cb_id = &attr
            .meta
            .require_name_value()
            .map_err(syn::Error::into_compile_error)?
            .value;
        quote_spanned! {span=>
            impl crate::packets::ClientboundPacket for #ident {
                const ID: u32 = #cb_id;
            }

        }
    } else {
        quote! {}
    };

    let mut asserts = Vec::new();
    let fields = struct_parse_fields(data_struct);

    let codes = fields
        .into_iter()
        .map(|(field, member)| {
            let span = field.span();
            let ty = field.ty.to_token_stream();

            let is_generic = if let syn::Type::Path(TypePath { path, .. }) = &field.ty
                && let Some(ident) = path.get_ident()
            {
                generics_types.contains(ident)
            } else {
                false
            };

            if !is_generic {
                asserts.push(assert_trait(
                    span,
                    field.ty.to_token_stream(),
                    quote_spanned! {span=> crate::data::Deserialize},
                ));
            }

            quote_spanned! {span=> #member: <#ty>::deserialize(stream)?}
        })
        .collect::<Vec<_>>();

    Ok( quote_spanned! {span=>
        #(#asserts)*

        #id_code

        impl #generics_with_params crate::data::Deserialize for #ident #generics {
            #[allow(unused_variables)]
            #[allow(clippy::init_numbered_fields)]
            fn deserialize(stream: &mut crate::data::DataStream) -> Result<Self, crate::data::DeserializeError> {
                Ok(Self {
                    #(#codes),*
                })
            }
        }
    }.into())
}

fn get_from_result<T>(r: Result<T, T>) -> T {
    match r {
        Ok(a) => a,
        Err(a) => a,
    }
}

fn common_checks_struct(input: &DeriveInput, _data_struct: &DataStruct) -> Result<(), TokenStream> {
    if let Some(attr) = get_attr(input, "enum_repr") {
        return Err(error(
            "This is an enum only attribute".to_string(),
            attr.span(),
        ));
    }

    Ok(())
}

fn commond_checks_enum(input: &DeriveInput, _data_enum: &DataEnum) -> Result<(), TokenStream> {
    if let Some(attr) = get_attr(input, "cb_id") {
        return Err(error(
            "This is an enum only attribute".to_string(),
            attr.span(),
        ));
    }
    if let Some(attr) = get_attr(input, "sb_id") {
        return Err(error(
            "This is an enum only attribute".to_string(),
            attr.span(),
        ));
    }

    Ok(())
}
