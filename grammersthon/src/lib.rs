#[macro_use] extern crate log;

use grammers_client::Client;
use grammers_client::types::User;
use trait_bound_typemap::{CloneSendSyncTypeMap, TypeMap};
use handler::Handlers;

pub use grammers_client;
pub use grammers_session;
pub use grammersthon_macro::{handler, FromArgs};
pub use crate::builder::GrammersthonBuilder;
pub use crate::error::GrammersthonError;
pub use crate::handler::{HandlerResult, HandlerFilter, Data, HandlerData, FromHandlerData};
pub use crate::args::{Args, FromArgs, RawArgs};

mod args;
mod error;
mod builder;
mod handler;

pub struct Grammersthon {
    client: Client,
    handlers: Handlers,
    me: User,
    data: CloneSendSyncTypeMap
}

impl Grammersthon {
    /// Create new builder instance
    pub fn new(api_id: i32, api_hash: &str) -> GrammersthonBuilder {
        GrammersthonBuilder::new(api_id, api_hash)
    }

    /// New builder instance from enviromnent variables (`TG_ID`, `TG_HASH`)
    pub fn from_env() -> Option<GrammersthonBuilder> {
        Some(GrammersthonBuilder::new(std::env::var("TG_ID").ok()?.parse().ok()?, &std::env::var("TG_HASH").ok()?))
    }

    /// Create new instance from client
    pub async fn from_client(mut client: Client) -> Result<Grammersthon, GrammersthonError> {
        Ok(Grammersthon {
            me: client.get_me().await?,
            client,
            handlers: Handlers::new(),
            data: CloneSendSyncTypeMap::new(),
        })
    }

    /// Get a client handle
    pub fn client(&self) -> Client {
        self.client.clone()
    }

    /// Get own user
    pub fn me(&self) -> &User {
        &self.me
    }

    /// Add custom data to use in handlers
    pub fn add_data<T: Send + Sync + Clone + 'static>(&mut self, data: T) -> &mut Self {
        self.data.insert::<Data<T>>(data);
        self
    }
    
    /// Run infinite event loop
    pub async fn start_event_loop(&mut self) -> Result<(), GrammersthonError> {
        info!("Starting event loop");
        loop {
            while let Some(update) = self.client.next_update().await? {
                // Run handler in own task
                let handlers = self.handlers.clone();
                let client = self.client.clone();
                let me = self.me.clone();
                let data = self.data.clone();
                tokio::task::spawn(async move {
                    match handlers.handle(client.clone(), update, me, data).await {
                        Ok(_) => (),
                        Err(e) => {
                            if let Err(e) = (*handlers.error)(e, client).await {
                                error!("Error occured while running error handler: {e}");
                            }
                        },
                    }
                });

            }
        }
    }
}