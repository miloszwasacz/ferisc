use proc_macro::TokenStream;
use quote::{quote, quote_spanned};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Block, Expr, Pat, Token, braced, parse_macro_input};

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
            let stage = &stage.0;
            quote_spanned! { stage.span() => {
                let __stage: PipelineStage<_> = #stage;
                __stage.execute();
            }}
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
                };
            }
        }
        Item::Custom(custom) => {
            let custom = &custom.0;
            quote! { #custom }
        }
    }
}

//#region Parsing

mod kw {
    syn::custom_keyword!(stage);
    syn::custom_keyword!(select);
    syn::custom_keyword!(custom);
}

struct Pipeline(pub Punctuated<Item, Token![,]>);

enum Item {
    Stage(Stage),
    Select(Select),
    Custom(Custom),
}

struct Stage(pub Block);

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
        } else if input.peek(kw::select) {
            input.parse().map(Item::Select)
        } else if input.peek(kw::custom) {
            input.parse().map(Item::Custom)
        } else {
            Err(input.error("expected `stage` or `select`"))
        }
    }
}

impl Parse for Stage {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse::<kw::stage>()?;
        input.parse().map(Self)
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
