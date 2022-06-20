#[macro_use] extern crate log;

use std::error::Error;
use grammersthon::grammers_client::{Client, types::Message};
use grammersthon::{Grammersthon, HandlerResult,  GrammersthonError};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    std::env::set_var("RUST_LOG", "info");
    pretty_env_logger::init();
    
    Grammersthon::from_env()
        .expect("Missing TG_ID or TG_HASH env variable")
        .interactive(true)
        .connect()
        .await?
        .fallback_handler(fallback)
        .error_handler(error_handler)
        .start_event_loop()
        .await?;

    Ok(())
}


/// Fallback handler, no #[handler] needed
async fn fallback(message: Message) -> HandlerResult {
    info!("Unhandled message: {}", message.text());
    Ok(())
}


/// Error handler, static parameters, no #[handler]
async fn error_handler(error: GrammersthonError, _client: Client) -> HandlerResult {
    error!("An error occured while handling: {error}");
    Ok(())
}