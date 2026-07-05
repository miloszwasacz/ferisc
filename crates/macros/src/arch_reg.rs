use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

pub fn derive(input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as syn::ItemEnum);
    let name = &item.ident;
    let count = item.variants.len();
    let hardwired = item
        .variants
        .iter()
        .filter(|v| {
            v.attrs
                .iter()
                .any(|attr| matches!(&attr.meta, syn::Meta::Path(p) if p.is_ident("hardwired")))
        })
        .map(|v| &v.ident)
        .collect::<Vec<_>>();
    let from = item
        .attrs
        .iter()
        .any(|attr| matches!(&attr.meta, syn::Meta::List(attr) if attr.path.is_ident("repr")))
        .then(|| {
            quote! {
                impl ::core::convert::From<#name> for usize {
                    fn from(value: #name) -> Self {
                        value as Self
                    }
                }
            }
        });

    quote! {
        impl ArchReg for #name {
            fn is_hardwired(&self) -> bool {
                match self {
                    #( Self::#hardwired => true, )*
                    _ => false,
                }
            }
        }

        impl ArchRegCount for #name {
            const COUNT: usize = #count;
            type COUNT = generic_array::ConstArrayLength<#count>;
        }

        #from
    }
    .into()
}
