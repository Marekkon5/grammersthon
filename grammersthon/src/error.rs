use std::fmt;
use grammers_client::client::chats::{AuthorizationError, InvocationError};
use grammers_client::client::SignInError;

#[derive(Debug)]
pub enum GrammersthonError {
    IO(std::io::Error),
    AuthorizationError(AuthorizationError),
    MissingParameters(&'static str),
    SignInError(SignInError),
    InvocationError(InvocationError),
    Unimplemented,
    Error(Box<dyn std::error::Error + Send + Sync>),
    Parse(String, Option<Box<dyn std::error::Error + Send + Sync>>)
}

impl fmt::Display for GrammersthonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GrammersthonError::IO(e) => write!(f, "IO error: {e}"),
            GrammersthonError::AuthorizationError(e) => write!(f, "Authorization error: {e}"),
            GrammersthonError::MissingParameters(e) => write!(f, "Missing parameters: {e}"),
            GrammersthonError::SignInError(e) => write!(f, "Sign in error: {e}"),
            GrammersthonError::InvocationError(e) => write!(f, "Other error: {e}"),
            GrammersthonError::Unimplemented => write!(f, "Unimplemented"),
            GrammersthonError::Error(e) => write!(f, "{e}"),
            GrammersthonError::Parse(value, e) => match e {
                Some(e) => write!(f, "Error parsing {value}: {e}"),
                None => write!(f, "Error parsing {value}")
            },
            
        }
    }
}

impl From<std::io::Error> for GrammersthonError {
    fn from(e: std::io::Error) -> Self {
        GrammersthonError::IO(e)
    }
}

impl From<AuthorizationError> for GrammersthonError {
    fn from(e: AuthorizationError) -> Self {
        GrammersthonError::AuthorizationError(e)
    }
}

impl From<SignInError> for GrammersthonError {
    fn from(e: SignInError) -> Self {
        GrammersthonError::SignInError(e)
    }
}

impl From<InvocationError> for GrammersthonError {
    fn from(e: InvocationError) -> Self {
        GrammersthonError::InvocationError(e)
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for GrammersthonError {
    fn from(e: Box<dyn std::error::Error + Send + Sync>) -> Self {
        GrammersthonError::Error(e)
    }
}

impl std::error::Error for GrammersthonError {}