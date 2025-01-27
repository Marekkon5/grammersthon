use proc_macro::TokenStream;
use quote::quote;
use regex::Regex;
use syn::punctuated::Punctuated;
use syn::{parse_macro_input, ItemFn, Result, Lit, ExprClosure, DeriveInput, Data, FieldsUnnamed, Ident, Fields, FieldsNamed, DataEnum, Attribute, Token};
use syn::parse::{ParseStream, Parse};

extern crate proc_macro;

/// Convert function into a handler function
/// ## Usage:
/// 
/// ### Single Regex pattern:
/// ```
/// #[handler("regex_pattern")]
/// ```
/// 
/// ### Single function:
/// `m` is `&Message`
/// `h` is`&HandlerData`
/// ```
/// #[handler(|m, h| true)]
/// ```
/// 
/// ### Combined:
/// 
/// ```
/// #[handler("regex", |m, h| true)]
/// ```
#[proc_macro_attribute]
pub fn handler(metadata: TokenStream, input: TokenStream) -> TokenStream {
    let filters = parse_macro_input!(metadata as HandlerFilters);
    let input_fn = parse_macro_input!(input as ItemFn);

    // Generate filters code
    let mut filters_code = vec![];
    for filter in filters.0 {
        let code = match filter {
            HandlerFilter::Regex(r) => quote! { ::grammersthon::HandlerFilter::Regex(#r.to_string()) },
            HandlerFilter::Fn(f) => quote! { ::grammersthon::HandlerFilter::Fn(::std::sync::Arc::new(::std::boxed::Box::new(#f))) },
        };
        filters_code.push(code);
    }

    // Function name
    let ident = input_fn.sig.ident.clone();
    let out = quote! {
        #input_fn

        #[doc(hidden)]
        #[allow(non_camel_case_types)]
        pub struct #ident {}

        impl #ident {
            #[allow(non_snake_case, unreachable_patterns, unreachable_code)]
            pub fn info() -> ::std::vec::Vec<::grammersthon::HandlerFilter> {
                ::std::vec![#(#filters_code),*]
            }
        }
    };

    TokenStream::from(out)
}

struct HandlerFilters(Vec<HandlerFilter>);

impl Parse for HandlerFilters {
    fn parse(input: ParseStream) -> Result<Self> {
        let filters = Punctuated::<HandlerFilter, Token![,]>::parse_separated_nonempty(input)?;
        Ok(HandlerFilters(filters.into_iter().collect()))
    }
}

enum HandlerFilter {
    Regex(String),
    Fn(ExprClosure)
}

impl Parse for HandlerFilter {
    fn parse(input: ParseStream) -> Result<Self> {
        // Try to parse as String pattern
        match Lit::parse(input) {
            Ok(Lit::Str(pattern)) => {
                // Validate
                let regex = pattern.value().to_string();
                Regex::new(&regex).expect("Invalid pattern regex!");
                return Ok(Self::Regex(regex));
            },
            _ => {}
        }

        // Parse as fn
        let closure = ExprClosure::parse(input)?;
        Ok(HandlerFilter::Fn(closure))
    }
}

/// Derive `FromArgs`
#[proc_macro_derive(FromArgs, attributes(rest, ignore_case))]
pub fn derive_from_args(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    match input.data {
        // Parse struct
        Data::Struct(s) => {
            // Parse fields
            let (field_count, out) = match s.fields {
                Fields::Named(f) => from_args_named_fields(&name, f),
                Fields::Unnamed(f) => from_args_unnamed_fields(&name, f),
                Fields::Unit => panic!("Unsupported struct type (Unit)"),
            };

            // Generate output impl
            let output = quote! {
                impl FromArgs for #name {
                    fn parse_arg(input: &::std::primitive::str) -> ::std::result::Result<#name, ::grammersthon::GrammersthonError> {
                        // Split
                        let (args, rest) = ::grammersthon::RawArgs::parse_n(input, #field_count);
                        if args.0.len() < #field_count {
                            return Err(::grammersthon::GrammersthonError::Parse(input.to_string(), None))
                        }
                        #out
                    }
                }

            };
            return TokenStream::from(output);
        },
        Data::Enum(e) => {
            let match_code = from_args_enum(&name, &e, &input.attrs);

            // Generate impl
            let output = quote! {
                impl FromArgs for #name {
                    fn parse_arg(input: &::std::primitive::str) -> ::std::result::Result<#name, ::grammersthon::GrammersthonError> {
                        #match_code
                    }
                }
            };
            return TokenStream::from(output);
        },
        _ => panic!("Unsupported data type")
    };



}

/// Parse struct with unnamed fields into FromArgs body
fn from_args_unnamed_fields(name: &Ident, fields: FieldsUnnamed) -> (usize, proc_macro2::TokenStream) {
    let mut count = fields.unnamed.len();
    let fields = fields.unnamed.iter().enumerate().map(|(i, f)| {
        let ty = &f.ty;
        // Check for #[rest] attribute
        let rest_attr = f.attrs.iter().any(|a| a.path().get_ident().map(|i| i.to_string().as_str() == "rest").unwrap_or(false));
        // Last field use rest
        if i == (count - 1) && rest_attr {
            count -= 1;
            quote! { <#ty>::parse_arg(&rest)? }
        } else {
            quote! { <#ty>::parse_arg(&args.0[#i])? }
        }
    }).collect::<Vec<_>>();
    let out = quote! { Ok(#name (#(#fields),*)) };
    (count, out)
}

/// Parse struct with named fields into FromArgs body
fn from_args_named_fields(name: &Ident, fields: FieldsNamed) -> (usize, proc_macro2::TokenStream) {
    let mut count = fields.named.len();
    let fields = fields.named.iter().enumerate().map(|(i, f)| {
        let ty = &f.ty;
        let name = f.ident.as_ref().unwrap();
        // Check for #[rest] attribute
        let rest_attr = f.attrs.iter().any(|a| a.path().get_ident().map(|i| i.to_string().as_str() == "rest").unwrap_or(false));
        // Last field use rest
        if i == (count - 1) && rest_attr {
            count -= 1;
            quote! { #name: <#ty>::parse_arg(&rest)? }
        } else {
            quote! { #name: <#ty>::parse_arg(&args.0[#i])? }
        }
    }).collect::<Vec<_>>();
    let out = quote! { Ok(#name { #(#fields),* }) };
    (count, out)
}

// Parse enum
fn from_args_enum(name: &Ident, e: &DataEnum, attributes: &Vec<Attribute>) -> proc_macro2::TokenStream {
    // Check if ignore case enabled
    let ignore_case = attributes.iter().any(|a| a.path().get_ident().map(|i| &i.to_string() == "ignore_case").unwrap_or(false));
    
    // Parse variants
    let options = e.variants.iter().map(|v| {
        let v_name = &v.ident;
        let mut v_name_str = v_name.to_string();
        if ignore_case {
            v_name_str = v_name_str.to_lowercase();
        }
        match v.fields {
            Fields::Unit => quote! { #v_name_str => Ok(#name::#v_name), },
            _ => panic!("Not supported yet!")
        }
    }).collect::<Vec<_>>();

    // If case should be ignored
    let input = match ignore_case {
        true => quote! { input.to_lowercase().as_str() },
        false => quote! { input }
    };

    quote! { 
        match #input { 
            #(#options)* 
            _ => Err(::grammersthon::GrammersthonError::Parse(input.to_string(), None))
        }
    }
}