# grammersthon

An attempt to turn Grammers into an user friendly framework.
Inspired by bevy, rocket, actix-web.

Grammers: https://github.com/Lonami/grammers

## Installing:
`Cargo.toml`:

```toml
grammersthon = { git = "https://github.com/Marekkon5/grammersthon.git" }
```

## Example

```rs
use std::error::Error;
use grammersthon::{Grammersthon, HandlerResult, handler, h};
use grammersthon::grammers_client::types::Message;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    Grammersthon::from_env().expect("Missing TG_ID or TG_HASH env variable")
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
```

For more examples see the `examples/` folder.