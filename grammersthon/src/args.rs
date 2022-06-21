use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::ffi::OsString;
use std::path::PathBuf;

use crate::{FromHandlerData, HandlerData, GrammersthonError};

/// Wrapper for parsing arguments from message body
pub struct Args<A: FromArgs>(pub A);

impl<A: FromArgs> FromHandlerData for Args<A> {
    fn from_data(data: &HandlerData) -> Option<Self> {
        let i = data.text.find(" ")?;
        Some(Args(A::parse_arg(&data.text[i..]).ok()?))
    }
}

/// Raw arguments (space separated, empty ignored)
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RawArgs(pub Vec<String>);

impl RawArgs {
    /// Parse n amount of arguments, return rest
    pub fn parse_n(input: &str, count: usize) -> (RawArgs, String) {
        // No args 
        if count == 0 {
            return (RawArgs::default(), input.to_string());
        }

        let mut args = vec![];
        let mut arg = String::new();

        // Split on space
        let mut chars = input.chars();
        for c in &mut chars {
            if c == ' ' {
                if !arg.is_empty() {
                    args.push(arg.trim().to_string());
                    arg.clear();
                    if args.len() == count { 
                        break;
                    }
                }
                continue;
            }
            arg.push(c);
        }

        // Rest
        if !arg.is_empty() {
            args.push(arg);
        }
        (RawArgs(args), chars.collect::<String>())
    }
}

impl FromArgs for RawArgs {
    fn parse_arg(input: &str) -> Result<Self, GrammersthonError> {
        Ok(RawArgs(input.split(" ").filter(|a| !a.trim().is_empty()).map(|a| a.trim().to_string()).collect::<Vec<_>>()))
    }
}

impl FromHandlerData for RawArgs {
    fn from_data(data: &HandlerData) -> Option<Self> {
        match data.text.find(" ") {
            Some(i) => RawArgs::parse_arg(&data.text[i..]).ok(),
            None => Some(RawArgs::default())
        }
    }
}

/// Can be parsed from message arguments
pub trait FromArgs where Self: Sized {
    /// Parse from argument string
    fn parse_arg(input: &str) -> Result<Self, GrammersthonError>;
}

impl FromArgs for String {
    fn parse_arg(input: &str) -> Result<Self, GrammersthonError> {
        Ok(input.to_string())
    }
}

impl FromArgs for bool {
    fn parse_arg(input: &str) -> Result<Self, GrammersthonError> {
        match input.trim().to_lowercase().as_str() {
            "true" | "yes" | "y" => Ok(true),
            "false" | "no" | "n" => Ok(false),
            _ => Err(GrammersthonError::Parse(input.to_string(), None))
        }
    }
}

impl<T: FromArgs> FromArgs for Vec<T> {
    fn parse_arg(input: &str) -> Result<Self, GrammersthonError> {
        let parts = RawArgs::parse_arg(input)?.0;
        let mut out = vec![];
        for part in parts {
            out.push(T::parse_arg(&part)?);
        }
        Ok(out)
    }
}

/// Generate FromArgs for primitive types
macro_rules! from_args_parse({ $($t:ty)* } => {
    $(impl FromArgs for $t {
        fn parse_arg(input: &str) -> Result<$t, GrammersthonError> {
            input.parse::<$t>().map_err(|e| GrammersthonError::Parse(input.to_string(), Some(e.into())))
        }
    })*
});

from_args_parse!(i8 u8 i16 u16 i32 u32 i64 u64 i128 u128 isize usize 
    f32 f64 char IpAddr Ipv4Addr Ipv6Addr OsString PathBuf);


/// Test RawArgs::parse_n
#[test]
fn test_parse_n() {
    let input = "aaa  bbb c d e  f  g";
    assert_eq!(RawArgs::parse_n(input, 0), (RawArgs::default(), input.to_string()));
    assert_eq!(RawArgs::parse_n(input, 1), (RawArgs(vec!["aaa".to_string()]), " bbb c d e  f  g".to_string()));
    assert_eq!(RawArgs::parse_n(input, 2), (RawArgs(vec!["aaa".to_string(), "bbb".to_string()]), "c d e  f  g".to_string()));
    assert_eq!(RawArgs::parse_n(input, 99), (RawArgs::parse_arg(input).unwrap(), String::new()));
}
