#[macro_use] extern crate log;

use grammers_client::Client;
use grammers_client::types::User;
use handler::Handlers;

pub use grammers_client;
pub use grammers_session;
pub use grammersthon_macro::handler;
pub use crate::builder::GrammersthonBuilder;
pub use crate::error::GrammersthonError;
pub use crate::handler::HandlerResult;

mod error;
mod builder;
mod handler;

pub struct Grammersthon {
    client: Client,
    handlers: Handlers,
    me: User
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
        })
    }

    /// Get a client handle
    pub fn client(&self) -> Client {
        self.client.clone()
    }

    /// Run infinite event loop
    pub async fn start_event_loop(&mut self) -> Result<(), GrammersthonError> {
        info!("Starting event loop");
        loop {
            while let Some(update) = self.client.next_update().await? {
                self.handlers.handle(self.client.clone(), update, self.me.clone()).await?;
            }
        }
    }
}