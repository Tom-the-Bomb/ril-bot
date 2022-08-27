use std::env;
use dotenv::dotenv;

use serenity::{
    prelude::*,
    async_trait,
    model::{
        prelude::UserId,
        gateway::Ready,
        channel::Message,
    },
    framework::standard::{
        HelpOptions,
        help_commands,
        macros::{hook, help, command, group},
        StandardFramework,
        CommandGroup,
        CommandResult,
        Args,
    },
};

use std::collections::HashSet;
use crate::utils::{
    functions::*,
    imaging::do_command,
};

mod utils;


#[group]
#[commands(invert)]
struct Imaging;

struct Handler;

struct ClientData;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _ctx: Context, data: Ready) {
        println!("Bot is ready!\nLogged in as {} ({})",
            data.user.tag(),
            data.user.id,
        );
    }
}

impl TypeMapKey for ClientData {
    type Value = reqwest::Client;
}

#[hook]
async fn error_handler(ctx: &Context, message: &Message, _cmd_name: &str, result: CommandResult) {
    if let Err(err) = result {
        message.reply(ctx, format!("{}", err))
            .await
            .ok();
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let framework = StandardFramework::new()
        .configure(
            |conf| conf.prefix("r!").with_whitespace(true)
        )
        .after(error_handler)
        .group(&IMAGING_GROUP)
        .help(&HELP_COMMAND);

    let token = env::var("TOKEN")
        .unwrap();

    let intents = GatewayIntents::non_privileged()
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(token, intents)
        .event_handler(Handler)
        .framework(framework)
        .await
        .unwrap();
    {
        let mut data = client.data.write().await;
        data.insert::<ClientData>(reqwest::Client::new());
    }

    client.start()
        .await
        .unwrap();
}


#[help]
async fn help_command(
    context: &Context,
    message: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    help_commands::with_embeds(
        context,
        message,
        args,
        help_options,
        groups,
        owners,
    )
        .await?;

    Ok(())
}


#[command]
async fn invert(ctx: &Context, message: &Message, args: Args) -> CommandResult {
    do_command(ctx, message, args, invert_func).await?;

    Ok(())
}