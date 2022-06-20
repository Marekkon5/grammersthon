use std::error::Error;
use grammersthon::grammers_client::types::Message;
use grammersthon::{Data, Grammersthon, HandlerResult, handler, h};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    std::env::set_var("RUST_LOG", "info");
    pretty_env_logger::init();
    
    // Your custom data
    let config = MyConfig { example: true };

    // Use `TG_ID` and `TG_HASH` env variables
    Grammersthon::from_env()
        .expect("Missing TG_ID or TG_HASH env variable")
        .session_file("session.session")?
        .interactive(true)
        .connect()
        .await?
        .add_data(config)
        .add_handler(h!(data))
        .start_event_loop()
        .await?;

    Ok(())
}

#[derive(Debug, Clone, Default)]
struct MyConfig {
    #[allow(dead_code)]
    pub example: bool
}

/// Will reply with the saved config
#[handler("/config")]
async fn data(message: Message, config: Data<MyConfig>) -> HandlerResult {
    let config = config.inner();
    message.reply(format!("{config:?}")).await?;
    Ok(())
}