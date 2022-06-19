/// Credits: 
/// 1. https://stackoverflow.com/questions/71083061/function-variable-with-dynamic-function-parameters
/// 2. https://stackoverflow.com/questions/68700171/how-can-i-assign-metadata-to-a-trait


use std::future::Future;
use std::pin::Pin;
use grammers_client::types::media::{Document, Sticker};
use grammers_client::{Update, Client};
use grammers_client::types::{Message, Media, Photo, User};
use grammers_tl_types::types::{MessageReplyHeader, MessageFwdHeader};
use regex::Regex;

use crate::{GrammersthonError, Grammersthon};

pub type HandlerResult = Result<(), GrammersthonError>;
pub type HandlerFn = dyn Fn(&HandlerData) -> Option<Pin<Box<dyn Future<Output = HandlerResult>>>>;

/// For registering handlers
#[macro_export]
macro_rules! h {
    ($a:ident) => {
        ($a::info(), $a)
    };
}

/// Default handler message
pub(crate) async fn default_fallback_handler(message: String) -> HandlerResult {
    warn!("Unhandled message: {message}");
    Ok(())
}

impl Grammersthon {
    /// Register event handler
    pub fn add_handler<F, A>(&mut self, handler: (&'static str, F)) -> &mut Self 
    where
        F: Handler<A>,
        A: FromHandlerData + 'static
    {
        let (pattern, handler) = handler;
        // Unwrap because it is compile checked in handler macro
        let pattern = Regex::new(pattern).unwrap();
        self.handlers.add(pattern, Handlers::box_handler(handler));
        self
    }

    /// Register fallback handler function
    pub fn fallback_handler<F, A>(&mut self, handler: F) -> &mut Self
    where
        F: Handler<A>,
        A: FromHandlerData + 'static
    {
        self.handlers.fallback = Handlers::box_handler(handler);
        self
    }
}

/// All the registered handlers
pub(crate) struct Handlers {
    fallback: Box<HandlerFn>,
    handlers: Vec<HandlerWrap>
}

/// Wrapper for handler with metadata
pub(crate) struct HandlerWrap {
    pub pattern: Regex,
    pub handler: Box<HandlerFn>
}

impl Handlers {
    /// Create new empty instance
    pub(crate) fn new() -> Handlers {
        Handlers {
            handlers: vec![],
            fallback: Self::box_handler(default_fallback_handler)
        }
    }

    /// Box handler fn
    fn box_handler<F, A>(handler: F) -> Box<HandlerFn>
    where
        F: Handler<A>,
        A: FromHandlerData + 'static
    {
        // Wrap handler with calling function
        let f = move |data: &HandlerData| -> Option<Pin<Box<dyn Future<Output = HandlerResult>>>> {
            Some(Box::pin(handler.call(A::from_data(data)?)))
        };
        Box::new(f)
    }

    /// Register new handler
    fn add(&mut self, pattern: Regex, handler: Box<HandlerFn>) {
        self.handlers.push(HandlerWrap { pattern, handler });
    }

    /// Handle incoming update
    pub(crate) async fn handle(&self, client: Client, update: Update, me: User) -> HandlerResult {
        let message = match update {
            Update::NewMessage(m) => m,
            u => {
                error!("Not implemented update type: {u:?}");
                return Err(GrammersthonError::Unimplemented);
            },
        };

        // Arguments
        let text = message.text().to_string();
        let data = HandlerData {
            text: text.to_string(),
            client, 
            message, 
            me
        };

        // Find handler
        for handler in &self.handlers {
            if handler.pattern.is_match(&text) {
                if let Some(f) = (*handler.handler)(&data) {
                    return f.await;
                }
            }
        }

        // Run fallback
        if let Some(f) = (*self.fallback)(&data) {
            return f.await;
        }
        Err(GrammersthonError::MissingParameters("Fallback handle function parameter"))
    }
}


/// Should contain all the data for Handler argument
pub struct HandlerData {
    client: Client,
    message: Message,
    text: String,
    me: User
}

/// For generating handlers
pub trait FromHandlerData {
    fn from_data(data: &HandlerData) -> Option<Self> where Self: Sized;
}

impl FromHandlerData for Client {
    fn from_data(data: &HandlerData) -> Option<Self> {
        Some(data.client.clone())
    }
}

impl FromHandlerData for Message {
    fn from_data(data: &HandlerData) -> Option<Self> {
        Some(data.message.clone())
    }
}

impl FromHandlerData for String {
    fn from_data(data: &HandlerData) -> Option<Self> {
        Some(data.text.clone())
    }
}

impl FromHandlerData for Media {
    fn from_data(data: &HandlerData) -> Option<Self> {
        data.message.media()
    }
}

impl FromHandlerData for Photo {
    fn from_data(data: &HandlerData) -> Option<Self> {
        data.message.photo()
    }
}

impl FromHandlerData for Document {
    fn from_data(data: &HandlerData) -> Option<Self> {
        data.message.media().map(|m| match m {
            Media::Document(d) => Some(d),
            _ => None
        }).flatten()
    }
}

impl FromHandlerData for Sticker {
    fn from_data(data: &HandlerData) -> Option<Self> {
        data.message.media().map(|m| match m {
            Media::Sticker(s) => Some(s),
            _ => None
        }).flatten()
    }
}

impl FromHandlerData for MessageReplyHeader {
    fn from_data(data: &HandlerData) -> Option<Self> {
        data.message.reply_header().map(|h| h.into())
    }
}

impl FromHandlerData for MessageFwdHeader {
    fn from_data(data: &HandlerData) -> Option<Self> {
        data.message.forward_header().map(|h| h.into())
    }
}

impl FromHandlerData for User {
    fn from_data(data: &HandlerData) -> Option<Self> {
        Some(data.me.clone())
    }
}

/// Generate FromHandlerData for n-tuple of FromHandlerData:
/// ```
/// impl<A: FromHandlerData + 'static> FromHandlerData for (A,) {
///     fn from_data(data: HandlerData) -> Self {
///         (A::from_data(data),)
///     }
/// }
/// ```
macro_rules! from_handler_data_impl({ $($param:ident)* } => {
    impl<$($param: FromHandlerData + 'static,)*> FromHandlerData for ($($param,)*) {
        #[allow(unused)]
        fn from_data(data: &HandlerData) -> Option<Self> {
            Some(($($param::from_data(data)?,)*))
        }
    }
});

from_handler_data_impl! { }
from_handler_data_impl! { A }
from_handler_data_impl! { A B }
from_handler_data_impl! { A B C }
from_handler_data_impl! { A B C D }
from_handler_data_impl! { A B C D E }
from_handler_data_impl! { A B C D E F }
from_handler_data_impl! { A B C D E F G }
from_handler_data_impl! { A B C D E F G H }


/// Trait of handler function
pub trait Handler<Args>: Clone + 'static {
    type Future: Future<Output = HandlerResult>;

    fn call(&self, args: Args) -> Self::Future;
}

/// Generates a [`Handler`] trait impl for N-ary functions where N is specified with a sequence of
/// space separated type parameters.
macro_rules! handler_fn({ $($param:ident)* } => {
    impl<Func, Fut, $($param,)*> Handler<($($param,)*)> for Func
    where 
        Func: Fn($($param),*) -> Fut + Clone + 'static,
        Fut: Future<Output = HandlerResult>
    {
        type Future = Fut;

        #[inline]
        #[allow(non_snake_case)]
        fn call(&self, ($($param,)*): ($($param,)*)) -> Self::Future {
            (self)($($param,)*)
        }
    }
});

handler_fn! { }
handler_fn! { A }
handler_fn! { A B }
handler_fn! { A B C }
handler_fn! { A B C D }
handler_fn! { A B C D E }
handler_fn! { A B C D E F }
handler_fn! { A B C D E F G }
handler_fn! { A B C D E F G H }
