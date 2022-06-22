#[macro_use] extern crate log;

use std::error::Error;
use grammersthon::grammers_client::{Update, Client, types::Message};
use grammersthon::{Grammersthon, HandlerResult,  GrammersthonError, HandlerData};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    std::env::set_var("RUST_LOG", "info");
    pretty_env_logger::init();
    
    Grammersthon::from_env()
        .expect("Missing TG_ID or TG_HASH env variable")
        .session_file("session.session")?
        .interactive(true)
        .connect()
        .await?

        // Fallback when no message handler is matched
        .message_fallback_handler(message_fallback)

        // Handle errors returned by handlers
        .error_handler(error_handler)

        // Called before handling message
        .interceptor(interceptor)

        // Handle any non-message updates there
        .fallback_handler(fallback)

        .start_event_loop()
        .await?;

    Ok(())
}


/// Fallback handler, no #[handler] needed
async fn message_fallback(message: Message) -> HandlerResult {
    info!("Unhandled message: {}", message.text());
    Ok(())
}


/// Error handler, static parameters, no #[handler]
async fn error_handler(error: GrammersthonError, _client: Client, update: Update) -> HandlerResult {
    error!("An error occured while handling: {update:?}: {error}");
    Ok(())
}

/// Here you can log any incoming message and it's HandlerData
/// Optionally edit HandlerData or return Err to cancel
async fn interceptor(data: HandlerData) -> Result<HandlerData, GrammersthonError> {
    info!("NewMessage event: {}", data.message.text());
    Ok(data)
}

/// Handle any non-message update
async fn fallback(_client: Client, update: Update) -> HandlerResult {
    info!("Unhandled update: {update:?}");
    Ok(())
}