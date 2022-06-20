use proc_macro::TokenStream;
use quote::quote;
use regex::Regex;
use syn::{parse_macro_input, ItemFn, Result, Lit, ExprClosure};
use syn::parse::{ParseStream, Parse};

extern crate proc_macro;

/// Handler function, usage:
/// `#[handler("regex_pattern")]`
#[proc_macro_attribute]
pub fn handler(metadata: TokenStream, input: TokenStream) -> TokenStream {
    let filter = parse_macro_input!(metadata as HandlerFilter);
    let input_fn = parse_macro_input!(input as ItemFn);

    let filter = match filter {
        HandlerFilter::Regex(r) => quote! { ::grammersthon::HandlerFilter::Regex(#r.to_string()) },
        HandlerFilter::Fn(f) => quote! { ::grammersthon::HandlerFilter::Fn(::std::sync::Arc::new(::std::boxed::Box::new(#f))) },
    };

    // Function name
    let ident = input_fn.sig.ident.clone();
    let out = quote! {
        #input_fn

        #[doc(hidden)]
        #[allow(non_camel_case_types)]
        pub struct #ident {}

        impl #ident {
            #[allow(non_snake_case, unreachable_patterns, unreachable_code)]
            fn info() -> ::grammersthon::HandlerFilter {
                #filter
            }
        }
    };

    TokenStream::from(out)
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
