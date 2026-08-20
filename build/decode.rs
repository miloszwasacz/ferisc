use convert_case::{Boundary, Case, Casing, separator};
use itertools::Itertools;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use std::{
    collections::HashMap,
    fs::{self, File},
    io::{BufRead, BufReader},
    path::Path,
    rc::Rc,
    str::FromStr,
};
use walkdir::{DirEntry, WalkDir};

const DECODE_DIR: &str = "decode";

pub fn gen_decode_fns<S, O>(spec_dir: S, out_dir: O)
where
    S: AsRef<Path>,
    O: AsRef<Path>,
{
    let decode_dir = spec_dir.as_ref().join(DECODE_DIR);
    let files = WalkDir::new(decode_dir)
        .into_iter()
        .map(Result::unwrap)
        .map(DirEntry::into_path)
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("enc")))
        .filter(|p| {
            p.file_stem()
                .is_none_or(|p| p.to_str().is_none_or(|p| !p.ends_with("common")))
        })
        .map(parse_enc_file);

    for (path, insts) in files {
        let path = path.with_extension("rs");
        let file_name = path.file_name().unwrap();
        let out_dir = out_dir.as_ref().join(DECODE_DIR);
        let out = out_dir.join(file_name);
        fs::create_dir_all(&out_dir).unwrap();

        let tokens = match insts {
            Ok(insts) => gen_decode_fn(insts),
            Err(e) => e,
        };

        let contents = syn::parse2::<syn::File>(tokens)
            .map(|f| prettyplease::unparse(&f))
            .unwrap_or_else(|e| {
                let e = format!("reformating error: {e}");
                quote! { compile_error!(#e); }.to_string()
            });

        fs::write(out, contents).unwrap();
    }
}

fn gen_decode_fn(insts: Vec<Inst>) -> TokenStream {
    macro_rules! int {
        ($len:expr, $sign:literal) => {{
            let len = $len.unwrap_or(32);
            syn::Ident::new(format!("{}{}", $sign, len).as_str(), Span::call_site())
        }};
    }

    let mut bytes_len = None;
    let mut matches = Vec::new();
    for group in insts.chunk_by(|a, b| a.mask == b.mask) {
        let match_arms = group
            .iter()
            .map(|inst| {
                assert!(
                    bytes_len
                        .replace(inst.pat.len() - 2)
                        .is_none_or(|len| len == group[0].pat.len() - 2)
                );

                let func = &inst.func;
                let fmt = &inst.fmt;
                let pat = syn::LitInt::new(&inst.pat, Span::call_site());
                quote! {
                    #pat => {
                        return Some(
                            <#fmt>::decode(bytes)
                                .map(|fmt| #func(fmt, meta, mode))
                                .unwrap_or(UopKind::Special(SpecialUopKind::Except(Exception::Illegal)))
                        );
                    }
                }
            })
            .collect::<Vec<_>>();

        let mask = format!("{:#0width$b}", group[0].mask, width = group[0].pat.len());
        let mask = syn::LitInt::new(mask.as_str(), Span::call_site());
        let cast_type = int!(bytes_len, "u");
        matches.push(quote! {
            match bytes as #cast_type & #mask {
                #( #match_arms )*
                _ => {}
            }
        })
    }
    let bytes_type = int!(bytes_len, "i");
    quote! {
        pub fn decode(bytes: #bytes_type, meta: Meta, mode: Mode) -> Option<UopKind> {
            #( #matches )*
            None
        }
    }
}

fn parse_enc_file<P: AsRef<Path>>(path: P) -> (P, Result<Vec<Inst>, TokenStream>) {
    macro_rules! r#try {
        ($result:expr, $path:expr) => {
            match $result {
                Ok(result) => result,
                Err(e) => {
                    let e = format!(
                        "parsing error in file `{}`: {}",
                        $path.as_ref().display(),
                        e
                    );
                    let tokens = quote! { compile_error!(#e); };
                    return ($path, Err(tokens));
                }
            }
        };
    }

    let file = r#try!(File::open(path.as_ref()), path);
    let mut defs = HashMap::<_, Inst>::new();
    for (i, line) in BufReader::new(file).lines().enumerate() {
        let line = r#try!(line, path);
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        } else if let Some(subpath) = line
            .strip_prefix("@include(\"")
            .and_then(|l| l.strip_suffix("\")"))
        {
            let Some(parent) = path.as_ref().parent() else {
                let message = format!("invalid include path: `{}`", subpath);
                return (path, Err(quote! { compile_error!(#message); }));
            };
            let sub_defs = match parse_enc_file(parent.join(subpath)).1 {
                Ok(defs) => defs,
                Err(e) => return (path, Err(e)),
            };
            defs.extend(sub_defs.into_iter().map(|i| ((i.mask, i.pat.clone()), i)));
        } else {
            let inst = match Inst::from_str(line) {
                Ok(inst) => inst,
                Err(e) => {
                    let message = format!("malformed line {i}: {e}");
                    return (path, Err(quote! { compile_error!(#message); }));
                }
            };
            defs.insert((inst.mask, inst.pat.clone()), inst);
        }
    }
    let defs = defs
        .into_values()
        .sorted_unstable_by_key(|inst| std::cmp::Reverse(inst.mask))
        .collect::<Vec<_>>();

    (path, Ok(defs))
}

type Mask = u32;

struct Inst {
    mask: Mask,
    pat: Rc<str>,
    fmt: syn::TypePath,
    func: syn::Ident,
}

impl FromStr for Inst {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (fmt, s) = s.split_once(";").ok_or("missing separator")?;
        let (pat_def, name) = s.split_once(";").ok_or("missing second separator")?;

        let fmt = syn::parse_str::<syn::TypePath>(fmt).map_err(|e| e.to_string())?;
        let name = name
            .set_boundaries(&[
                Boundary::Hyphen,
                Boundary::Underscore,
                Boundary::Space,
                separator!("."),
            ])
            .to_case(Case::Snake);

        let mut mask = 0;
        let mut pat = String::from("0b");
        for c in pat_def.chars() {
            if c == '|' {
                continue;
            }
            mask <<= 1;
            match c {
                '0' | '1' => {
                    mask |= 1;
                    pat.push(c);
                }
                'X' => {
                    pat.push('0');
                }
                _ => Err("unknown character in a pattern (allowed characters: 0,1,X)")?,
            }
        }
        Ok(Self {
            mask,
            pat: pat.into(),
            fmt,
            func: syn::Ident::new(name.as_str(), Span::call_site()),
        })
    }
}
