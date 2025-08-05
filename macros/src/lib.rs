#![allow(clippy::uninlined_format_args)]

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::{quote, quote_spanned};
use syn::{
    AngleBracketedGenericArguments, Attribute, Data, DataEnum, DataStruct, DeriveInput, Expr,
    ExprLit, Field, Fields, FieldsNamed, FieldsUnnamed, GenericArgument, Lit, LitInt, Member,
    PathArguments, PredicateType, Token, Type, TypePath, WherePredicate, parse_macro_input,
    parse_quote, punctuated::Punctuated, spanned::Spanned,
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

fn type_contains_ident(ty: &Type, ident: &Ident) -> bool {
    match ty {
        Type::Path(TypePath { path, .. }) => {
            if path.is_ident(ident) {
                return true;
            }

            for segment in &path.segments {
                if let PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                    args, ..
                }) = &segment.arguments
                {
                    for arg in args {
                        match arg {
                            GenericArgument::Type(ty) if type_contains_ident(ty, ident) => {
                                return true;
                            }
                            _ => (),
                        }
                    }
                }
            }

            false
        }
        _ => false,
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
    let ident = &input.ident;

    let mut generics = input.generics.clone();
    let where_clause = generics.make_where_clause();

    let repr: Ident = if let Some(attr) = get_attr(input, "enum_repr") {
        let list = attr
            .meta
            .require_list()
            .map_err(syn::Error::into_compile_error)?;
        list.parse_args().map_err(syn::Error::into_compile_error)?
    } else {
        return Err(error("Missing enum_repr attribute".to_string(), span));
    };

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
            if !type_contains_ident(&field.ty, ident) {
                let mut bounds = Punctuated::new();
                bounds.push(parse_quote!(crate::data::Serialize));
                where_clause
                    .predicates
                    .push(WherePredicate::Type(PredicateType {
                        bounded_ty: field.ty.clone(),
                        bounds,
                        colon_token: Token![:](span),
                        lifetimes: None,
                    }));
            }
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

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    Ok((quote_spanned! {span=>
        #[allow(clippy::just_underscores_and_digits)]
        impl #impl_generics crate::data::Serialize for #ident #ty_generics #where_clause {
            fn size(&self) -> usize {
                match self {
                    #(#size_lines),*
                }
            }


            fn serialize(&self, stream: &mut dyn std::io::Write) -> Result<(), crate::data::SerializeError> {
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
    let name = &input.ident;

    let mut generics = input.generics.clone();
    let where_clause = generics.make_where_clause();

    let id_code = if let Some(attr) = get_attr(input, "sb_id") {
        let sb_id = &attr
            .meta
            .require_name_value()
            .map_err(syn::Error::into_compile_error)?
            .value;
        quote_spanned! {span=>
            impl crate::packets::ServerboundPacket for #name {
                const ID: u32 = #sb_id;
            }

        }
    } else {
        quote! {}
    };

    let fields = struct_parse_fields(data_struct);

    let (field_codes_size, field_codes_serialize): (Vec<_>, Vec<_>) = fields
        .into_iter()
        .map(|(field, member)| {
            let span = member.span();

            if !type_contains_ident(&field.ty, name) {
                let mut bounds = Punctuated::new();
                bounds.push(parse_quote!(crate::data::Serialize));
                where_clause
                    .predicates
                    .push(WherePredicate::Type(PredicateType {
                        bounded_ty: field.ty,
                        bounds,
                        colon_token: Token![:](span),
                        lifetimes: None,
                    }));
            }

            let size = quote_spanned! {span=> n += self.#member.size();};
            let serialize = quote_spanned! {span=> self.#member.serialize(stream)?;};

            (size, serialize)
        })
        .unzip();

    let ident = &input.ident;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    Ok(quote_spanned! {span=>
        #id_code

        impl #impl_generics crate::data::Serialize for #ident #ty_generics #where_clause {
            #[allow(unused_mut, clippy::let_and_return)]
            fn size(&self) -> usize {
                let mut n = 0;
                #(#field_codes_size)*
                n
            }

            #[allow(unused_variables)]
            fn serialize(&self, stream: &mut dyn std::io::Write) -> Result<(), crate::data::SerializeError> {
                #(#field_codes_serialize)*
                Ok(())
            }
        }
    }.into())
}

#[proc_macro_derive(Deserialize, attributes(enum_repr))]
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
    let ident = &input.ident;

    let mut generics = input.generics.clone();
    let where_clause = generics.make_where_clause();

    let repr: Ident = if let Some(attr) = get_attr(input, "enum_repr") {
        let list = attr
            .meta
            .require_list()
            .map_err(syn::Error::into_compile_error)?;
        list.parse_args().map_err(syn::Error::into_compile_error)?
    } else {
        return Err(error("Missing enum_repr attribute".to_string(), span));
    };

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
            if !type_contains_ident(&field.ty, ident) {
                let mut bounds = Punctuated::new();
                bounds.push(parse_quote!(crate::data::Deserialize));
                where_clause
                    .predicates
                    .push(WherePredicate::Type(PredicateType {
                        bounded_ty: field.ty.clone(),
                        bounds,
                        colon_token: Token![:](span),
                        lifetimes: None,
                    }));
            }
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

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    Ok((quote_spanned! {span=>

        impl #impl_generics crate::data::Deserialize for #ident #ty_generics #where_clause {
            fn deserialize(stream: &mut crate::data::DataStream) -> Result<Self, crate::data::DeserializeError> {
                let repr = <#repr>::deserialize(stream)?;
                match crate::utils::macros::EnumRepr::to_value(repr) {
                    #(#deserialize_lines,)*
                    other => Err(crate::data::DeserializeError::MalformedPacket(format!("{}: invalid type {}", stringify!(#ident), other)))
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
    let name = &input.ident;

    let mut generics = input.generics.clone();
    let where_clause = generics.make_where_clause();

    let fields = struct_parse_fields(data_struct);

    let codes = fields
        .into_iter()
        .map(|(field, member)| {
            let span = field.span();
            let ty = field.ty.clone();

            if !type_contains_ident(&field.ty, name) {
                let mut bounds = Punctuated::new();
                bounds.push(parse_quote!(crate::data::Deserialize));
                where_clause
                    .predicates
                    .push(WherePredicate::Type(PredicateType {
                        bounded_ty: field.ty,
                        bounds,
                        colon_token: Token![:](span),
                        lifetimes: None,
                    }));
            }

            quote_spanned! {span=> #member: <#ty>::deserialize(stream)?}
        })
        .collect::<Vec<_>>();

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    Ok( quote_spanned! {span=>
        impl #impl_generics crate::data::Deserialize for #name #ty_generics #where_clause {
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
