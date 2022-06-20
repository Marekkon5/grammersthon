#[macro_use] extern crate log;

use std::error::Error;
use grammersthon::grammers_client::types::{Media, User, Message, Chat};
use grammersthon::grammers_client::types::media::Sticker;
use grammersthon::grammers_client::{Client, InputMessage};
use grammersthon::{Grammersthon, HandlerResult, handler, h};
use regex::Regex;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    std::env::set_var("RUST_LOG", "info");
    pretty_env_logger::init();

    // Login
    let mut grammersthon = Grammersthon::from_env()
        .expect("Missing TG_ID or TG_HASH env variable")
        .interactive(true)
        .session_file("session.session")?
        .connect()
        .await?;

    // Save session
    grammersthon.client().session().save_to_file("session.session")?;

    // Add handlers
    grammersthon
        // Add pattern mutator which will prefix `/` to every pattern
        .pattern_mutator(|pattern| Regex::new(&format!("/{pattern}")).unwrap())

        // Register individual handlers
        .add_handler(h!(ping))
        .add_handler(h!(save_media))
        .add_handler(h!(with_sticker))
        .add_handler(h!(fn_handler_example))

        // Fallback handler for unhandled messages
        .fallback_handler(fallback)

        // Start
        .start_event_loop()
        .await?;

    Ok(())
}

/// Will handle only messages with Stickers
#[handler(|_| true)]
async fn with_sticker(_sticker: Sticker) -> HandlerResult {
    info!("Message with Sticker received!");
    Ok(())
}

/// Will reply to any message with the content `/ping`
#[handler("ping$")]
async fn ping(message: Message) -> HandlerResult {
    message.reply("Pong!").await?;
    Ok(())
}

/// Will reupload any message with Media and `save` as text to Saved Messages
#[handler("save$")]
async fn save_media(client: Client, me: User, media: Media) -> HandlerResult {
    client.send_message(me, InputMessage::text("Saved!").copy_media(&media)).await?;
    Ok(())
}

/// Only handle messages of people with usernames
#[handler(|m| matches!(m.chat(), Chat::User(u) if u.username().is_some() ))]
async fn fn_handler_example(message: Message) -> HandlerResult {
    info!("Message from user with username: {message:?}");
    Ok(())
}

/// Fallback handler, no #[handler] needed
async fn fallback(message: Message) -> HandlerResult {
    info!("Unhandled message: {}", message.text());
    Ok(())
}