use proc_macro::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;

pub fn proc_macro(input: TokenStream) -> TokenStream {
    let syn::ItemEnum {
        mut attrs,
        vis,
        enum_token: _,
        ident,
        generics: _,
        brace_token: _,
        variants,
    } = syn::parse_macro_input!(input as syn::ItemEnum);
    let repr = unwrap!(
        attrs
            .iter()
            .enumerate()
            .find_map(|(i, attr)| match &attr.meta {
                syn::Meta::List(list) if list.path.is_ident("repr") => {
                    Some(list.parse_args::<syn::Type>().map(|ty| (i, ty)))
                }
                _ => None,
            })
            .transpose()
    )
    .map(|(i, ty)| {
        attrs.remove(i);
        ty
    });

    let vars = unwrap!(
        variants
            .iter()
            .enumerate()
            .map(|(i, var)| {
                let syn::Variant {
                    attrs,
                    ident,
                    fields,
                    discriminant,
                } = &var;
                if fields.is_empty() && discriminant.is_none() {
                    let const_ident =
                        syn::Ident::new(ident.to_string().to_uppercase().as_str(), ident.span());
                    let r#const = quote_spanned! {var.span() =>
                        #( #attrs )*
                        const #const_ident = 1 << #i;
                    };

                    let fn_ident =
                        syn::Ident::new(ident.to_string().to_lowercase().as_str(), ident.span());
                    let r#fn = quote_spanned! {var.span() =>
                        #( #attrs )*
                        pub fn #fn_ident(&self) -> bool {
                            self.contains(Self::#const_ident)
                        }
                    };

                    Ok((r#const, r#fn))
                } else {
                    Err(syn::Error::new(
                        var.fields.span(),
                        "no fields or discriminants are allowed",
                    ))
                }
            })
            .collect::<Result<Vec<_>, _>>()
    );
    let (consts, fns): (Vec<_>, Vec<_>) = vars.into_iter().unzip();

    quote! {
        ::bitflags::bitflags! {
            #( #attrs )*
            #vis struct #ident: #repr {
                #( #consts )*
            }
        }

        impl #ident {
            #( #fns )*
        }
    }
    .into()
}
