use std::error::Error;
use grammersthon::grammers_client::types::Message;
use grammersthon::{Grammersthon, HandlerResult, handler, h};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    std::env::set_var("RUST_LOG", "info");
    pretty_env_logger::init();
    
    // Use `TG_ID` and `TG_HASH` env variables
    Grammersthon::from_env()
        .expect("Missing TG_ID or TG_HASH env variable")
        // Prompt in shell for auth
        .interactive(true)
        .connect()
        .await?
        .add_handler(h!(ping))
        .start_event_loop()
        .await?;

    Ok(())
}

/// Will reply to any message with the content `Ping!`
#[handler("^Ping!$")]
async fn ping(message: Message) -> HandlerResult {
    message.reply("Pong!").await?;
    Ok(())
}