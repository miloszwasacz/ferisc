macro_rules! unwrap {
    ($result:expr) => {{
        match $result {
            Ok(x) => x,
            Err(e) => return e.to_compile_error().into(),
        }
    }};
}

macro_rules! create_derive {
    ($name:ident, attributes($( $attrs:tt )*)) => {
        ::paste::paste! {
            mod [<$name:snake>];

            #[proc_macro_derive($name, attributes($( $attrs )*))]
            pub fn [<derive_ $name:snake>](input: ::proc_macro::TokenStream) -> ::proc_macro::TokenStream {
                [<$name:snake>]::derive(input)
            }
        }
    };
    ($name:ident) => {
        create_derive!($name, attributes());
    }
}

create_derive!(ArchReg, attributes(hardwired));
