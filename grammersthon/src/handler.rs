/// Credits: 
/// 1. https://stackoverflow.com/questions/71083061/function-variable-with-dynamic-function-parameters
/// 2. https://stackoverflow.com/questions/68700171/how-can-i-assign-metadata-to-a-trait


use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use grammers_client::types::media::{Document, Sticker};
use grammers_client::{Update, Client};
use grammers_client::types::{Message, Media, Photo, User, Chat, Group, Channel};
use grammers_tl_types::types::{MessageReplyHeader, MessageFwdHeader, MessageReplyStoryHeader};
use regex::Regex;
use trait_bound_typemap::{CloneSendSyncTypeMap, TypeMapKey, TypeMap};

use crate::{GrammersthonError, Grammersthon};

pub type HandlerResult = Result<(), GrammersthonError>;
type HandlerFn = dyn Fn(&HandlerData) -> Option<Pin<Box<dyn Future<Output = HandlerResult> + Send + Sync>>> + Send + Sync;
type ErrorHandlerFn = dyn Fn(GrammersthonError, Client, Update) -> Pin<Box<dyn Future<Output = HandlerResult> + Send + Sync>> + Send + Sync;
type PatternMutatorFn = dyn Fn(&str) -> Regex + Send + Sync;
type InterceptorFn = dyn Fn(HandlerData) -> Pin<Box<dyn Future<Output = Result<HandlerData, GrammersthonError>> + Send + Sync>> + Send + Sync;
type FallbackFn = dyn Fn(Client, Update) -> Pin<Box<dyn Future<Output = HandlerResult> + Send + Sync>> + Send + Sync;

/// For registering handlers
#[macro_export]
macro_rules! h {
    ($a:ident) => {
        ($a::info(), $a)
    };
}

/// Default fallback handler
pub(crate) async fn default_message_fallback_handler(message: String) -> HandlerResult {
    warn!("Unhandled message: {message}");
    Ok(())
}

impl Grammersthon {
    /// Register event handler
    pub fn add_handler<F, A>(&mut self, handler: (Vec<HandlerFilter>, F)) -> &mut Self 
    where
        F: Handler<A>,
        A: FromHandlerData + 'static
    {
        let (filters, handler) = handler;
        self.handlers.add(filters, Handlers::box_handler(handler));
        self
    }

    /// Register message fallback handler function
    /// Will be called if no NewMessage handler will be matched
    pub fn message_fallback_handler<F, A>(&mut self, handler: F) -> &mut Self
    where
        F: Handler<A>,
        A: FromHandlerData + 'static
    {
        self.handlers.message_fallback = Handlers::box_handler(handler);
        self
    }

    /// Register handler for all events other than NewMessage
    pub fn fallback_handler<H, F>(&mut self, handler: H) -> &mut Self 
    where
        H: (Fn(Client, Update) -> F) + Send + Sync + 'static,
        F: Future<Output = HandlerResult> + Send + Sync + 'static
    {
        self.handlers.fallback = Arc::new(Box::new(move |c, u| {
            Box::pin(handler(c, u))
        }));
        self
    }

    /// Register error handler
    pub fn error_handler<H, F>(&mut self, handler: H) -> &mut Self 
    where
        H: Fn(GrammersthonError, Client, Update) -> F + Send + Sync + 'static,
        F: Future<Output = HandlerResult> + Send + Sync + 'static
    {
        self.handlers.error = Arc::new(Box::new(move |e, c, u| {
            Box::pin(handler(e, c, u))
        }));
        self
    }

    /// Register pattern mutator function
    pub fn pattern_mutator<M>(&mut self, mutator: M) -> &mut Self 
    where
        M: (Fn(&str) -> Regex) + Send + Sync + 'static
    {
        self.handlers.pattern_mutator = Some(Arc::new(Box::new(mutator)));
        self
    }

    /// Register interceptor called before handling message
    pub fn interceptor<I, F>(&mut self, interceptor: I) -> &mut Self
    where
        I: (Fn(HandlerData) -> F) + Send + Sync + 'static,
        F: Future<Output = Result<HandlerData, GrammersthonError>> + Send + Sync + 'static
    {
        self.handlers.interceptor = Some(Arc::new(Box::new(move |d| {
            Box::pin(interceptor(d))
        })));
        self
    }
}

/// All the registered handlers
#[derive(Clone)]
pub(crate) struct Handlers {
    message_fallback: Arc<Box<HandlerFn>>,
    fallback: Arc<Box<FallbackFn>>,
    handlers: Vec<HandlerWrap>,
    pub error: Arc<Box<ErrorHandlerFn>>,
    pattern_mutator: Option<Arc<Box<PatternMutatorFn>>>,
    interceptor: Option<Arc<Box<InterceptorFn>>>,
}

/// Whether the handler should be executed or no
#[derive(Clone)]
pub enum HandlerFilter {
    Regex(String),
    Fn(Arc<Box<dyn Fn(&Message, &HandlerData) -> bool + Send + Sync>>)
}

impl HandlerFilter {
    /// Does the filter match 
    pub fn is_match(&self, message: &Message, mutator: &Option<Arc<Box<PatternMutatorFn>>>, data: &HandlerData) -> bool {
        match self {
            // Unwrap because regex is compile checked
            HandlerFilter::Regex(r) => {
                match mutator {
                    Some(mutator) => (*mutator)(r).is_match(message.text()),
                    None => Regex::new(&r).unwrap().is_match(message.text()),
                }
            },
            HandlerFilter::Fn(f) => (*f)(message, data),
        }
    }
}

/// Wrapper for handler with metadata
#[derive(Clone)]
pub(crate) struct HandlerWrap {
    pub filters: Vec<HandlerFilter>,
    pub handler: Arc<Box<HandlerFn>>
}

impl Handlers {
    /// Create new empty instance
    pub(crate) fn new() -> Handlers {
        Handlers {
            handlers: vec![],
            message_fallback: Self::box_handler(default_message_fallback_handler),
            pattern_mutator: None,
            interceptor: None,
            // Default error handler
            error: Arc::new(Box::new(|e, __, ___| { Box::pin(async move { 
                error!("Unhandled error occured: {e}");
                Ok(()) 
            }) })),
            // Default update fallback
            fallback: Arc::new(Box::new(|_, u| { Box::pin(async move {
                error!("Unhandled Update: {u:?}");
                Ok(())
            }) })),
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
    fn add(&mut self, filters: Vec<HandlerFilter>, handler: Arc<Box<HandlerFn>>) {
        self.handlers.push(HandlerWrap { filters, handler });
    }

    /// Handle incoming update
    pub(crate) async fn handle(&self, client: Client, update: Update, me: User, data: CloneSendSyncTypeMap) -> HandlerResult {
        let message = match update {
            Update::NewMessage(m) => m,
            update => {
                return (*self.fallback)(client, update).await;
            },
        };

        // Arguments
        let mut data = HandlerData { client, data, me, message: message.clone() };

        // Run interceptor
        if let Some(interceptor) = &self.interceptor {
            data = (*interceptor)(data).await?;
        }

        // Find handler
        for handler in &self.handlers {
            // Run all filters
            let matched = handler.filters.iter().all(|f| f.is_match(&message, &self.pattern_mutator, &data));
            if matched {
                if let Some(f) = (*handler.handler)(&data) {
                    return f.await;
                }
            }
        }

        // Run fallback
        if let Some(f) = (*self.message_fallback)(&data) {
            return f.await;
        }
        Err(GrammersthonError::MissingParameters("Fallback handle function parameter"))
    }

}


/// Should contain all the data for Handler argument
#[derive(Clone)]
pub struct HandlerData {
    pub client: Client,
    pub message: Message,
    pub me: User,
    pub data: CloneSendSyncTypeMap
}

impl HandlerData {
    /// Get any data added with .add_data
    pub fn data<T: Clone + Send + Sync + 'static>(&self) -> Option<T> {
        self.data.get::<Data<T>>().map(|t| t.clone())
    }
}

/// Wrapper for querying user data
#[derive(Clone)]
pub struct Data<T: Send + Sync + Clone>(pub T);

impl<T: Send + Sync + Clone> Data<T> {
    /// Get inner value
    pub fn inner(self) -> T {
        self.0
    }
}

impl<T: Send + Sync + Clone + 'static> TypeMapKey for Data<T> {
    type Value = T;
}

/// For querying self from args
#[derive(Debug, Clone)]
pub struct Me(pub User);

/// For generating handler function parameters
pub trait FromHandlerData: where Self: Sized {
    fn from_data(data: &HandlerData) -> Option<Self>;
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
        Some(data.message.text().to_string())
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
        data.message.reply_header().map(|h| match h {
            grammers_tl_types::enums::MessageReplyHeader::Header(h) => Some(h),
            grammers_tl_types::enums::MessageReplyHeader::MessageReplyStoryHeader(_) => None,
        }).flatten()
    }
}

impl FromHandlerData for MessageReplyStoryHeader {
    fn from_data(data: &HandlerData) -> Option<Self> {
        data.message.reply_header().map(|h| match h {
            grammers_tl_types::enums::MessageReplyHeader::Header(_) => None,
            grammers_tl_types::enums::MessageReplyHeader::MessageReplyStoryHeader(h) => Some(h),
        }).flatten()
    }
}

impl FromHandlerData for MessageFwdHeader {
    fn from_data(data: &HandlerData) -> Option<Self> {
        data.message.forward_header().map(|h| h.into())
    }
}

impl FromHandlerData for Chat {
    fn from_data(data: &HandlerData) -> Option<Self> {
        Some(data.message.chat())
    }
}

impl FromHandlerData for User {
    fn from_data(data: &HandlerData) -> Option<Self> {
        match data.message.chat() {
            Chat::User(u) => Some(u),
            Chat::Group(_) => None,
            Chat::Channel(_) => None,
        }
    }
}

impl FromHandlerData for Group {
    fn from_data(data: &HandlerData) -> Option<Self> {
        match data.message.chat() {
            Chat::User(_) => None,
            Chat::Group(g) => Some(g),
            Chat::Channel(_) => None,
        }
    }
}

impl FromHandlerData for Channel {
    fn from_data(data: &HandlerData) -> Option<Self> {
        match data.message.chat() {
            Chat::User(_) => None,
            Chat::Group(_) => None,
            Chat::Channel(c) => Some(c),
        }
    }
}

impl<T: Send + Sync + Clone + 'static> FromHandlerData for Data<T> {
    fn from_data(data: &HandlerData) -> Option<Self> {
        data.data.get::<Data<T>>().map(|t| Data(t.clone()))
    }
}

impl FromHandlerData for Me {
    fn from_data(data: &HandlerData) -> Option<Self> {
        Some(Me(data.me.clone()))
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
