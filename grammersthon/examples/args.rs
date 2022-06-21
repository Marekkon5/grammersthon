#[macro_use] extern crate log;

use std::error::Error;
use grammers_client::{types::Message, Client};
use grammersthon::{Grammersthon, HandlerResult, FromArgs, Args, handler, h, RawArgs};

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
        .add_handler(h!(hi))
        .add_handler(h!(sum))
        .add_handler(h!(repeat))
        .add_handler(h!(action))
        .add_handler(h!(any_args))
        .start_event_loop()
        .await?;

    Ok(())
}


/// Requires only one string as parameter, rest will be ignored
#[derive(FromArgs)]
struct Name(String);

/// Will reply with Hi and name from argument
#[handler("/hi")]
async fn hi(message: Message, args: Args<Name>) -> HandlerResult {
    let name = args.0.0;
    message.reply(format!("Hi {name}!")).await?;
    Ok(())
}


/// Requires u32, rest of the message will be inside of `text`
#[derive(Debug, Clone, FromArgs)]
struct RepeatArgs {
    amount: u32,
    #[rest]
    text: String,
}

/// Will repeat n times the text
#[handler("/repeat")]
async fn repeat(client: Client, message: Message, args: Args<RepeatArgs>) -> HandlerResult {
    let RepeatArgs { amount, text } = args.0;
    for _ in 0..amount {
        client.send_message(message.chat(), &*text).await?;
    }
    Ok(())
}

/// Enum example (only unit variants are supported)
#[derive(Debug, FromArgs)]
#[ignore_case]
enum Action {
    Play, Pause, Skip
}

/// Wrapper because without it the enum would never get matched due to first arg == function
#[derive(Debug, FromArgs)]
struct ActionArgs(Action);

/// Enum example 
#[handler("/action")]
async fn action(args: Args<ActionArgs>) -> HandlerResult {
    let action = args.0.0;
    info!("Player: {action:?}");
    Ok(())
}


/// Requires any amount of numbers
#[derive(Debug, Clone, FromArgs)]
struct Sum(#[rest] Vec<f32>);

/// Will sum all numbers
#[handler("/sum")]
async fn sum(message: Message, args: Args<Sum>) -> HandlerResult {
    let sum = args.0.0.iter().fold(0.0, |acc, x| acc + x);
    message.reply(format!("{sum}")).await?;
    Ok(())
}


/// Handle any arguments
#[handler("/args")]
async fn any_args(message: Message, args: RawArgs) -> HandlerResult {
    message.reply(args.0.join("\n")).await?;
    Ok(())
}