use std::env;
use dotenv::dotenv;

use serenity::{
    prelude::*,
    async_trait,
    model::{channel::Message, prelude::AttachmentType},
    framework::standard::{
        macros::{command, group},
        StandardFramework,
        CommandResult,
        Args,
    },
};

use crate::utils::ImageResolver;

mod utils;


#[group]
#[commands(test)]
struct General;

struct Handler;

#[async_trait]
impl EventHandler for Handler {}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let framework = StandardFramework::new()
        .configure(|conf| conf.prefix("r!"))
        .group(&GENERAL_GROUP);

    let token = env::var("TOKEN")
        .unwrap();

    let intents = GatewayIntents::non_privileged()
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(token, intents)
        .framework(framework)
        .await
        .unwrap();

    client.start()
        .await
        .unwrap();
}


#[command]
async fn test(ctx: &Context, message: &Message, mut args: Args) -> CommandResult {
    let resolved = ImageResolver::new()
        .resolve(ctx, message, &mut args)
        .await?;

    message.channel_id.send_message(ctx,
        |m| {
            m.add_file(
                AttachmentType::Bytes {
                    data: resolved.into(),
                    filename: "test.png".to_string(),
                }
            )
        }
    ).await?;

    Ok(())
}