use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::TokenStream as Tokens;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields, parse_macro_input, parse_quote};

/// Serialize fields in declaration order; enums use a zero-based VarUint tag.
#[proc_macro_derive(Serializable)]
pub fn derive_serializable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn expand(input: DeriveInput) -> syn::Result<Tokens> {
    let path =
        match crate_name("octet").map_err(|error| syn::Error::new_spanned(&input.ident, error))? {
            FoundCrate::Itself => quote!(::octet),
            FoundCrate::Name(name) => {
                let name = format_ident!("{name}");
                quote!(::#name)
            }
        };
    let name = &input.ident;
    let mut generics = input.generics.clone();
    let fields: Vec<_> = match &input.data {
        Data::Struct(data) => vec![&data.fields],
        Data::Enum(data) => data
            .variants
            .iter()
            .map(|variant| &variant.fields)
            .collect(),
        Data::Union(_) => {
            return Err(syn::Error::new_spanned(
                name,
                "Serializable cannot be derived for unions",
            ));
        }
    };
    for field in fields.iter().flat_map(|fields| fields.iter()) {
        let ty = &field.ty;
        generics
            .make_where_clause()
            .predicates
            .push(parse_quote!(#ty: #path::Serializable + ::core::marker::Sized));
    }

    let (write, read, size) = match &input.data {
        Data::Struct(data) => {
            let (pattern, writes, reads, sizes) = field_code(&data.fields, &path);
            (
                quote!(let Self #pattern = self; #writes),
                quote!(Ok(Self #reads)),
                quote!({ let Self #pattern = self; #sizes }),
            )
        }
        Data::Enum(data) => {
            let mut writes = Vec::new();
            let mut reads = Vec::new();
            let mut sizes = Vec::new();
            for (index, variant) in data.variants.iter().enumerate() {
                let tag = index as u64;
                let name = &variant.ident;
                let label = name.to_string();
                let (pattern, write, read, size) = field_code(&variant.fields, &path);
                writes.push(quote!(Self::#name #pattern => {
                    #path::Serializable::write(&#path::VarUint::from(#tag), writer).context("tag")?;
                    (|| -> Result<(), #path::Error> { #write Ok(()) })().context(#label)?;
                }));
                reads.push(quote!(#tag => (|| -> Result<Self, #path::Error> { Ok(Self::#name #read) })().context(#label),));
                sizes.push(
                    quote!(Self::#name #pattern => #path::VarUint::encoded_size(#index) + (#size),),
                );
            }
            (
                quote!(match self { #(#writes)* }),
                quote!(match <#path::VarUint as #path::Serializable>::read(reader).context("tag")?.0 {
                    #(#reads)*
                    _ => Err(#path::DecodeError::UnexpectedValue.into()),
                }),
                quote!(match self { #(#sizes)* }),
            )
        }
        Data::Union(_) => unreachable!(),
    };
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    Ok(quote! {
        impl #impl_generics #path::Serializable for #name #ty_generics #where_clause {
            fn write<__OctetSink: #path::Writable>(&self, writer: &mut #path::Writer<__OctetSink>) -> Result<(), #path::Error> {
                use #path::__private::ResultExt as _;
                writer.write_with_context::<Self>(|writer| { #write Ok(()) })
            }
            fn read<__OctetSource: #path::Readable>(reader: &mut #path::Reader<__OctetSource>) -> Result<Self, #path::Error> {
                use #path::__private::ResultExt as _;
                reader.read_with_context(|reader| { #read })
            }
            fn size(&self) -> usize {
                #size
            }
        }
    })
}

fn field_code(fields: &Fields, path: &Tokens) -> (Tokens, Tokens, Tokens, Tokens) {
    let bindings: Vec<_> = (0..fields.len())
        .map(|index| format_ident!("__octet_field_{index}"))
        .collect();
    let mut writes = Vec::new();
    let mut reads = Vec::new();
    for (index, (field, binding)) in fields.iter().zip(&bindings).enumerate() {
        let ty = &field.ty;
        let label = field
            .ident
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| index.to_string());
        writes.push(quote!(#path::Serializable::write(#binding, writer).context(#label)?;));
        reads.push(quote!(<#ty as #path::Serializable>::read(reader).context(#label)?));
    }
    let (pattern, read) = match fields {
        Fields::Named(fields) => {
            let names: Vec<_> = fields.named.iter().map(|field| &field.ident).collect();
            (
                quote!({ #(#names: #bindings),* }),
                quote!({ #(#names: #reads),* }),
            )
        }
        Fields::Unnamed(_) => (quote!((#(#bindings),*)), quote!((#(#reads),*))),
        Fields::Unit => (quote!(), quote!()),
    };
    let bound_sizes = bindings
        .iter()
        .map(|binding| quote!(#path::Serializable::size(#binding)));
    let size = quote!(0usize #(+ #bound_sizes)*);
    (pattern, quote!(#(#writes)*), read, size)
}
