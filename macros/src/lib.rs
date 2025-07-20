use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{ToTokens, quote, quote_spanned};
use syn::{
    Data, DataStruct, DeriveInput, Expr, Field, Fields, Ident, MetaNameValue,
    parse_macro_input, spanned::Spanned,
};

fn error(msg: String, span: Span) -> TokenStream {
    quote_spanned! {span=> compile_error!(#msg);}.into()
}

fn parse_fields(input: &DeriveInput) -> Result<impl IntoIterator<Item = Field>, TokenStream> {
    match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => Ok(fields.named.clone()),
        _ => Err(error("Not a Struct".to_string(), input.span())),
    }
}

fn get_attr(input: &DeriveInput, attr_name: &str) -> Option<Expr> {
    input.attrs.iter().find_map(|attr| match &attr.meta {
        syn::Meta::NameValue(MetaNameValue { value, path, .. })
            if path.get_ident().is_some_and(|ident| ident == attr_name) =>
        {
            Some(value.clone())
        }
        _ => None,
    })
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

#[proc_macro_derive(Serialize, attributes(sb_id))]
pub fn serialize_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let span = input.span();
    let ident = input.ident.clone();

    let fields = match parse_fields(&input) {
        Ok(fields) => fields,
        Err(tokens) => return tokens,
    };

     let id_code = if let Some(sb_id) = get_attr(&input, "sb_id") {
        quote_spanned! {span=>
            impl crate::packets::ServerboundPacket for #ident {
                const ID: u32 = #sb_id;
            }

        }
    } else {
        quote! {}
    };

    let mut asserts = Vec::new();

    let (field_codes_size, field_codes_serialize): (Vec<_>, Vec<_>) = fields
        .into_iter()
        .map(|field| {
            let span = field.span();
            let ident = field.ident.expect("All fields are named");

            asserts.push(assert_trait(
                span,
                field.ty.to_token_stream(),
                quote_spanned! {span=> crate::data::Serialize},
            ));

            let size = quote_spanned! {span=> n += self.#ident.size();};
            let serialize = quote_spanned! {span=> self.#ident.serialize(stream)?;};

            (size, serialize)
        })
        .unzip();

    let ident = input.ident;

    quote_spanned! {span=>
        #(#asserts)*

        #id_code

        impl crate::data::Serialize for #ident {
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
    }.into()
}

#[proc_macro_derive(Deserialize, attributes(cb_id))]
pub fn deserialize_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let span = input.span();
    let ident = input.ident.clone();

    let fields = match parse_fields(&input) {
        Ok(fields) => fields,
        Err(tokens) => return tokens,
    };

    let id_code = if let Some(cb_id) = get_attr(&input, "cb_id") {
        quote_spanned! {span=>
            impl crate::packets::ClientboundPacket for #ident {
                const ID: u32 = #cb_id;
            }

        }
    } else {
        quote! {}
    };

    let mut asserts = Vec::new();

    let codes = fields
        .into_iter()
        .map(|field| {
            let span = field.span();
            let ident = field.ident.expect("All fields are named");
            let ty = field.ty.to_token_stream();

            asserts.push(assert_trait(
                span,
                field.ty.to_token_stream(),
                quote_spanned! {span=> crate::data::Deserialize},
            ));

            quote_spanned! {span=> #ident: <#ty>::deserialize(stream)?}
        })
        .collect::<Vec<_>>();

    quote_spanned! {span=>
        #(#asserts)*

        #id_code

        impl crate::data::Deserialize for #ident {
            #[allow(unused_variables)]
            fn deserialize(stream: &mut crate::data::DataStream) -> Result<Self, crate::data::DeserializeError> {
                Ok(Self {
                    #(#codes),*
                })
            }
        }
    }.into()
}

// #[proc_macro_attribute]
// pub fn id(attribute: TokenStream, item: TokenStream) -> TokenStream {
//     let attr = syn::parse::<LitInt>(attribute);
//     let item = parse_macro_input!(item as DeriveInput);
//     let span = item.span();
//     let attr = match attr {
//         Ok(v) => v,
//         Err(e) => return e.into_compile_error().into(),
//     };

//     let ident = item.ident.clone();
//     quote_spanned! {span=>
//         #item
//         impl crate::packets::Packet for #ident {
//             const ID: u32 = #attr;
//         }
//     }
//     .into()
// }
