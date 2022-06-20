/// Credits: 
/// 1. https://stackoverflow.com/questions/71083061/function-variable-with-dynamic-function-parameters
/// 2. https://stackoverflow.com/questions/68700171/how-can-i-assign-metadata-to-a-trait


use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use grammers_client::types::media::{Document, Sticker};
use grammers_client::{Update, Client};
use grammers_client::types::{Message, Media, Photo, User};
use grammers_tl_types::types::{MessageReplyHeader, MessageFwdHeader};
use regex::Regex;

use crate::{GrammersthonError, Grammersthon};

pub type HandlerResult = Result<(), GrammersthonError>;
type HandlerFn = dyn Fn(&HandlerData) -> Option<Pin<Box<dyn Future<Output = HandlerResult> + Send + Sync>>> + Send + Sync;
type ErrorHandlerFn = dyn Fn(GrammersthonError, Client) -> Pin<Box<dyn Future<Output = HandlerResult> + Send + Sync>> + Send + Sync;

/// For registering handlers
#[macro_export]
macro_rules! h {
    ($a:ident) => {
        ($a::info(), $a)
    };
}

/// Default fallback handler
pub(crate) async fn default_fallback_handler(message: String) -> HandlerResult {
    warn!("Unhandled message: {message}");
    Ok(())
}

impl Grammersthon {
    /// Register event handler
    pub fn add_handler<F, A>(&mut self, handler: (HandlerFilter, F)) -> &mut Self 
    where
        F: Handler<A>,
        A: FromHandlerData + 'static
    {
        let (filter, handler) = handler;
        self.handlers.add(filter, Handlers::box_handler(handler));
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

    /// Register error handler
    pub fn error_handler<H, F>(&mut self, handler: H) -> &mut Self 
    where
        H: Fn(GrammersthonError, Client) -> F + Send + Sync + 'static,
        F: Future<Output = HandlerResult> + Send + Sync + 'static
    {
        self.handlers.error = Arc::new(Box::new(move |e, c| {
            Box::pin(handler(e, c))
        }));
        self
    }
}

/// All the registered handlers
#[derive(Clone)]
pub(crate) struct Handlers {
    fallback: Arc<Box<HandlerFn>>,
    handlers: Vec<HandlerWrap>,
    pub error: Arc<Box<ErrorHandlerFn>>,
}

/// Whether the handler should be executed or no
#[derive(Clone)]
pub enum HandlerFilter {
    Regex(String),
    Fn(Arc<Box<dyn Fn(&Message) -> bool + Send + Sync>>)
}

impl HandlerFilter {
    /// Does the filter match 
    pub fn is_match(&self, message: &Message) -> bool {
        match self {
            // Unwrap because regex is compile checked
            HandlerFilter::Regex(r) => Regex::new(&r).unwrap().is_match(message.text()),
            HandlerFilter::Fn(f) => (*f)(message),
        }
    }
}

/// Wrapper for handler with metadata
#[derive(Clone)]
pub(crate) struct HandlerWrap {
    pub filter: HandlerFilter,
    pub handler: Arc<Box<HandlerFn>>
}

impl Handlers {
    /// Create new empty instance
    pub(crate) fn new() -> Handlers {
        Handlers {
            handlers: vec![],
            fallback: Self::box_handler(default_fallback_handler),
            // Default error handler
            error: Arc::new(Box::new(|e, __| { Box::pin(async move { 
                error!("Unhandled error occured: {e}");
                Ok(()) 
            }) }))
        }
    }

    /// Box handler fn
    fn box_handler<F, A>(handler: F) -> Arc<Box<HandlerFn>>
    where
        F: Handler<A>,
        A: FromHandlerData + 'static
    {
        // Wrap handler with calling function
        let f = move |data: &HandlerData| -> Option<Pin<Box<dyn Future<Output = HandlerResult> + Send + Sync>>> {
            Some(Box::pin(handler.call(A::from_data(data)?)))
        };
        Arc::new(Box::new(f))
    }

    /// Register new handler
    fn add(&mut self, filter: HandlerFilter, handler: Arc<Box<HandlerFn>>) {
        self.handlers.push(HandlerWrap { filter, handler });
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
        let data = HandlerData {
            text: message.text().to_string(),
            client, 
            message: message.clone(), 
            me
        };

        // Find handler
        for handler in &self.handlers {
            if handler.filter.is_match(&message) {
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
#[derive(Debug, Clone)]
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
pub trait Handler<Args>: Send + Sync + Clone + 'static {
    type Future: Future<Output = HandlerResult> + Send + Sync;

    fn call(&self, args: Args) -> Self::Future;
}

/// Generates a [`Handler`] trait impl for N-ary functions where N is specified with a sequence of
/// space separated type parameters.
macro_rules! handler_fn({ $($param:ident)* } => {
    impl<Func, Fut, $($param,)*> Handler<($($param,)*)> for Func
    where 
        Func: Fn($($param),*) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = HandlerResult> + Send + Sync
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
