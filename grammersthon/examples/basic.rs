use std::error::Error;
use grammers_client::{types::{Media, media::Sticker, User, Message}, Client, InputMessage};
use grammersthon::{Grammersthon, HandlerResult, handler, h};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    Grammersthon::from_env()
        .expect("Missing TG_ID or TG_HASH env variable")
        .interactive(true)
        .connect()
        .await?
        .add_handler(h!(with_sticker))
        .add_handler(h!(save_media))
        .fallback_handler(fallback)
        .start_event_loop()
        .await?;

    Ok(())
}

/// Will handle only messages with Stickers
#[handler(".*")]
async fn with_sticker(_sticker: Sticker) -> HandlerResult {
    println!("Message with Sticker received!");
    Ok(())
}

/// Will reupload any message with Media and `save` as text to Saved Messages
#[handler("^save$")]
async fn save_media(client: Client, me: User, media: Media) -> HandlerResult {
    client.send_message(me, InputMessage::text("Saved!").copy_media(&media)).await?;
    Ok(())
}

/// Fallback handler, no #[handler] needed
async fn fallback(message: Message) -> HandlerResult {
    println!("Unhandled message: {}", message.text());
    Ok(())
}