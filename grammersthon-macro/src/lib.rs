use proc_macro::TokenStream;
use quote::quote;
use regex::Regex;
use syn::{LitStr, parse_macro_input, ItemFn};

extern crate proc_macro;

/// Handler function, usage:
/// `#[handler("regex_pattern")]`
#[proc_macro_attribute]
pub fn handler(metadata: TokenStream, input: TokenStream) -> TokenStream {
    let pattern = parse_macro_input!(metadata as LitStr);
    let input_fn = parse_macro_input!(input as ItemFn);

    // Validate regex
    Regex::new(&pattern.value()).expect("Invalid pattern regex!");

    // Function name
    let ident = input_fn.sig.ident.clone();
    let out = quote! {
        #input_fn

        #[doc(hidden)]
        #[allow(non_camel_case_types)]
        pub struct #ident {}

        impl #ident {
            #[allow(non_snake_case, unreachable_patterns, unreachable_code)]
            fn info() -> &'static ::std::primitive::str {
                #pattern
            }
        }
    };

    TokenStream::from(out)
}