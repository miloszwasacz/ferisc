use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Block, Expr, Pat, Token, braced, bracketed, parse_macro_input, token};

pub fn proc_macro(input: TokenStream) -> TokenStream {
    let pipeline = parse_macro_input!(input as Pipeline);
    quote_pipeline(&pipeline).into()
}

fn quote_pipeline(pipeline: &Pipeline) -> proc_macro2::TokenStream {
    let stages = pipeline.0.iter().rev().map(quote_item);
    quote! {{ #( #stages )* }}
}

fn quote_item(item: &Item) -> proc_macro2::TokenStream {
    match item {
        Item::Stage(stage) => {
            let decls = quote_decls(&stage.decls);
            let body = &stage.body;
            quote! {
                {
                    #( #decls )*
                    let __stage: PipelineStage<_> = #body;
                    __stage.execute();
                }
            }
        }
        Item::With(with) => {
            let decls = quote_decls(&with.decls);
            let pipeline = quote_pipeline(&with.pipeline);
            quote! {
                {
                    #( #decls )*
                    #pipeline
                }
            }
        }
        Item::Select(select) => {
            let expr = &select.expr;
            let arms = select.arms.iter().map(|arm| {
                let pat = &arm.pat;
                let body = match &arm.body {
                    SelectBody::Item(item) => quote_item(item),
                    SelectBody::SubPipeline(pipeline) => quote_pipeline(pipeline),
                };
                quote! { #pat => #body }
            });
            quote! {
                match #expr {
                    #( #arms ),*
                }
            }
        }
        Item::Custom(custom) => {
            let custom = &custom.0;
            quote! {{ #custom }}
        }
    }
}

fn quote_decls(decls: &Decls) -> impl Iterator<Item = proc_macro2::TokenStream> {
    decls.iter().map(|Decl { pat, init }| {
        quote! { let #pat = #init; }
    })
}

//#region Parsing

mod kw {
    syn::custom_keyword!(stage);
    syn::custom_keyword!(with);
    syn::custom_keyword!(select);
    syn::custom_keyword!(custom);
}

struct Pipeline(pub Punctuated<Item, Token![,]>);

enum Item {
    Stage(Stage),
    With(With),
    Select(Select),
    Custom(Custom),
}

struct Stage {
    pub decls: Decls,
    pub body: Block,
}

struct With {
    pub decls: Decls,
    pub pipeline: Pipeline,
}

type Decls = Punctuated<Decl, Token![,]>;

struct Decl {
    pub pat: Pat,
    pub init: Expr,
}

struct Select {
    pub expr: Expr,
    pub arms: Punctuated<SelectArm, Token![,]>,
}

struct SelectArm {
    pub pat: Pat,
    pub body: SelectBody,
}

enum SelectBody {
    Item(Item),
    SubPipeline(Pipeline),
}

struct Custom(pub Block);

impl Parse for Pipeline {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Punctuated::parse_terminated(input).map(Self)
    }
}

impl Parse for Item {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(kw::stage) {
            input.parse().map(Item::Stage)
        } else if input.peek(kw::with) {
            input.parse().map(Item::With)
        } else if input.peek(kw::select) {
            input.parse().map(Item::Select)
        } else if input.peek(kw::custom) {
            input.parse().map(Item::Custom)
        } else {
            Err(input.error("expected `stage`, `with`, `select`, or `custom`"))
        }
    }
}

impl Parse for Stage {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse::<kw::stage>()?;
        let decls = parse_decls(&input)?;
        let body = input.parse()?;

        Ok(Self { decls, body })
    }
}

impl Parse for With {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse::<kw::with>()?;
        let decls = parse_decls(&input)?;
        let content;
        braced!(content in input);
        let pipeline = content.parse()?;

        Ok(Self { decls, pipeline })
    }
}

fn parse_decls(input: &ParseStream) -> syn::Result<Decls> {
    if input.peek(token::Bracket) {
        let decls;
        bracketed!(decls in input);
        Punctuated::parse_terminated(&decls)
    } else {
        Ok(Punctuated::new())
    }
}

impl Parse for Decl {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let pat = Pat::parse_multi(input)?;
        input.parse::<token::Eq>()?;
        let init = input.parse()?;

        Ok(Self { pat, init })
    }
}

impl Parse for Select {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse::<kw::select>()?;
        let expr = input.parse()?;

        let content;
        braced!(content in input);
        let arms = Punctuated::parse_terminated(&content)?;

        Ok(Self { expr, arms })
    }
}

impl Parse for SelectArm {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let pat = Pat::parse_multi(input)?;
        input.parse::<Token![=>]>()?;
        let body = input.parse()?;

        Ok(Self { pat, body })
    }
}

impl Parse for SelectBody {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(syn::token::Brace) {
            let content;
            braced!(content in input);
            content.parse().map(Self::SubPipeline)
        } else {
            input.parse().map(Self::Item)
        }
    }
}

impl Parse for Custom {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse::<kw::custom>()?;
        input.parse().map(Self)
    }
}

//#endregion
