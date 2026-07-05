use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, spanned::Spanned};

pub fn proc_macro(input: TokenStream) -> TokenStream {
    let syn::DeriveInput {
        attrs,
        vis,
        ident,
        generics,
        data,
    } = parse_macro_input!(input as syn::DeriveInput);
    if !matches!(data, syn::Data::Struct(s) if s.fields.is_empty()) {
        return syn::Error::new(ident.span(), "only empty structs are supported")
            .into_compile_error()
            .into();
    }
    let where_clause = {
        if !generics.params.is_empty() {
            return syn::Error::new(generics.span(), "generics are not allowed")
                .into_compile_error()
                .into();
        }
        generics.where_clause
    };

    let (attrs, spec) = unwrap!(csr::extract_spec(attrs));
    let meta_impls = spec.iter().map(|spec| {
        let name = spec.name();
        let addr = spec.addr();
        let display = spec.display_name();
        quote! {
            impl CsrMeta for #name {
                const ADDR: CsrAddr = CsrAddr::new(#addr).unwrap();
                const NAME: &str = #display;
            }
        }
    });
    let fields = spec.iter().map(|spec| {
        let name = spec.field_name();
        let ty = spec.name();
        if spec.alias().is_none() {
            quote! { pub #name: CsrBuf<#ty>, }
        } else {
            quote! { pub #name: #ty, }
        }
    });
    let access_arms = spec.iter().map(|spec| {
        let ty = spec.name();
        let field = spec.field_name();
        match spec.alias() {
            None => quote! {
                #ty::ADDR => {
                    let csr = &mut self.#field;
                    let current = csr.read();
                    let current_int = current.as_int();
                    let new = match op {
                        CsrOp::Rw => data,
                        CsrOp::Rs => current_int | data,
                        CsrOp::Rc => current_int & !data,
                    };
                    csr.write(current.new_value(new)?);
                    Ok(current_int)
                }
            },
            Some(aliased) => quote! {
                #ty::ADDR => {
                    let target = &mut self.#aliased;
                    let current = target.read();
                    let current_int = #ty::alias_as_int(#ty::alias(current));
                    let new = match op {
                        CsrOp::Rw => data,
                        CsrOp::Rs => current_int | data,
                        CsrOp::Rc => current_int & !data,
                    };
                    target.write(current.new_value(#ty::dealias_int(new))?);
                    Ok(current_int)
                }
            },
        }
    });
    let commit_arms = spec.iter().map(|spec| {
        let ty = spec.name();
        let field = spec.field_name();
        match spec.alias() {
            None => quote! {
                #ty::ADDR => self.#field.commit(),
            },
            Some(aliased) => quote! {
                #ty::ADDR => self.#aliased.commit(),
            },
        }
    });
    let flush_arms = spec.iter().map(|spec| {
        let ty = spec.name();
        let field = spec.field_name();
        match spec.alias() {
            None => quote! {
                #ty::ADDR => self.#field.flush(),
            },
            Some(aliased) => quote! {
                #ty::ADDR => self.#aliased.flush(),
            },
        }
    });
    let finish_cycle = spec
        .iter()
        .filter(|spec| spec.alias().is_none())
        .map(|spec| {
            let field = spec.field_name();
            quote! { self.#field.finish_cycle(); }
        });

    let stats_fields = spec.iter().map(|spec| {
        let name = spec.field_name();
        let ty = spec.name();
        if spec.alias().is_none() {
            quote! { pub #name: #ty, }
        } else {
            quote! { pub #name: <#ty as CsrAlias>::Alias, }
        }
    });
    let stats = spec.iter().map(|spec| {
        let name = spec.field_name();
        let ty = spec.name();
        match spec.alias() {
            None => quote! { #name: self.#name.read().clone() },
            Some(target) => quote! {
                #name: <#ty as CsrAlias>::alias(self.#target.read()).clone()
            },
        }
    });

    quote! {
        #( #attrs )*
        #vis struct #ident #where_clause {
            #( #fields )*
        }

        impl #ident #where_clause {
            pub fn access(&mut self, csr: CsrAddr, op: CsrOp, data: IntVal) -> Result<IntVal, Exception> {
                match csr {
                    #( #access_arms )*
                    _ => Err(Exception::Illegal),
                }
            }

            pub fn commit(&mut self, csr: CsrAddr) {
                match csr {
                    #( #commit_arms )*
                    _ => panic!("unimplemented csr"),
                }
            }

            pub fn flush(&mut self, csr: CsrAddr) {
                match csr {
                    #( #flush_arms )*
                    _ => panic!("unimplemented csr"),
                }
            }
        }

        impl Sequential for #ident #where_clause {
            fn finish_cycle(&mut self) {
                #( #finish_cycle )*
            }
        }

        #[derive(Debug, Clone)]
        #vis struct Stats {
            #( #stats_fields )*
        }

        impl #ident #where_clause {
            pub fn stats(&self) -> Stats {
                Stats {
                    #( #stats ),*
                }
            }
        }

        #( #meta_impls )*
    }.into()
}

mod csr {
    use convert_case::{Case, Casing};
    use proc_macro2::Span;
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    pub fn extract_spec(
        attrs: Vec<syn::Attribute>,
    ) -> syn::Result<(Vec<syn::Attribute>, Vec<Csr>)> {
        let mut spec = None;
        let attrs = attrs
            .into_iter()
            .filter(|attr| match &attr.meta {
                syn::Meta::NameValue(attr) if attr.path.is_ident("spec_file") => {
                    match &attr.value {
                        syn::Expr::Lit(lit) => match &lit.lit {
                            syn::Lit::Str(path) => {
                                let span = path.span();
                                spec = Some(
                                    File::open(path.value())
                                        .map_err(|e| {
                                            syn::Error::new(
                                                span,
                                                format!("cannot open file: {}", e),
                                            )
                                        })
                                        .and_then(|f| read_spec_file(f, span)),
                                );
                                false
                            }
                            _ => true,
                        },
                        _ => true,
                    }
                }
                _ => true,
            })
            .collect::<Vec<_>>();
        let spec = spec.unwrap_or_else(|| {
            Err(syn::Error::new(
                Span::call_site(),
                "no specification file provided",
            ))
        })?;
        Ok((attrs, spec))
    }

    fn read_spec_file(file: File, span: Span) -> syn::Result<Vec<Csr>> {
        BufReader::new(file)
            .lines()
            .enumerate()
            .filter(|(_, l)| !l.as_ref().is_ok_and(|l| l.trim_start().starts_with("#")))
            .map(|(i, l)| {
                l.map_err(|e| syn::Error::new(span, format!("IO error at line {}: {}", i, e)))
                    .and_then(|l| Csr::parse(l.as_str(), i, span))
            })
            .collect::<syn::Result<Vec<_>>>()
    }

    pub struct Csr {
        addr: syn::LitInt,
        ident: syn::Ident,
        display: String,
        field: syn::Ident,
        alias: Option<syn::Ident>,
    }

    impl Csr {
        pub fn parse(line: &str, line_number: usize, span: Span) -> syn::Result<Self> {
            let (left, alias) = match line.split_once("=>") {
                Some((left, right)) => {
                    let ident = syn::Ident::new(right.trim().to_case(Case::Snake).as_str(), span);
                    (left.trim(), Some(ident))
                }
                None => (line.trim(), None),
            };
            let (addr, name) = left.split_once(',').ok_or_else(|| {
                syn::Error::new(
                    span,
                    format!(
                        "malformed input at line {}: expected format: addr,name or addr,name => alias",
                        line_number
                    ),
                )
            })?;
            let addr = addr.trim();
            let addr = addr
                .strip_prefix("0x")
                .ok_or_else(|| {
                    format!(
                        "malformed input at line {}: CSR address should be in hex",
                        line_number
                    )
                })
                .and_then(|addr| {
                    usize::from_str_radix(addr, 16).map_err(|e| {
                        format!("address parsing error at line {}: {}", line_number, e)
                    })
                })
                .map_err(|e| syn::Error::new(span, e))
                .map(|_| syn::LitInt::new(addr, span))?;

            let name = name.trim();
            let ident = syn::Ident::new(name.to_case(Case::Pascal).as_str(), span);
            let display = name.to_lowercase();
            let field = syn::Ident::new(name.to_case(Case::Snake).as_str(), span);

            Ok(Self {
                addr,
                ident,
                display,
                field,
                alias,
            })
        }

        pub fn addr(&self) -> &syn::LitInt {
            &self.addr
        }

        pub fn name(&self) -> &syn::Ident {
            &self.ident
        }

        pub fn display_name(&self) -> &str {
            &self.display
        }

        pub fn field_name(&self) -> &syn::Ident {
            &self.field
        }

        pub fn alias(&self) -> Option<&syn::Ident> {
            self.alias.as_ref()
        }
    }
}
