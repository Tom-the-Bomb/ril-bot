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
        buckets::LimitedFor,
        StandardFramework,
        CommandGroup,
        CommandResult,
        Args,
    },
};

use std::collections::HashSet;

#[allow(clippy::wildcard_imports)]
use crate::utils::{
    functions::*,
    imaging::ImageExecutor,
    helpers::{resolve_extra_arg, resolve_arg},
    resolver::ImageResolver,
};

mod utils;


#[group]
#[commands(
    invert,
    huerotate,
    caption,
)]
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

/// an "after" callback hook on commands to handle `Err` CommandResults and send the error message
#[hook]
async fn error_handler(ctx: &Context, message: &Message, _cmd_name: &str, result: CommandResult) {
    if let Err(err) = result {
        message.reply(ctx, err.to_string())
            .await
            .ok();
    }
}

/// a callback for when the user is still on cooldown when invoking a command
#[hook]
async fn delay_action(ctx: &Context, message: &Message) {
    message.reply(ctx, "⏲️ You are still on cooldown!")
        .await
        .ok();
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let framework = StandardFramework::new()
        .configure(
            |conf| conf.prefix("r!")
                .with_whitespace(true)
        )
        .after(error_handler)
        .group(&IMAGING_GROUP)
        .help(&HELP_COMMAND)
        .bucket("imaging",
            |bucket|
                bucket.delay(5)
                    .limit_for(LimitedFor::User)
                    .await_ratelimits(5)
                    .delay_action(delay_action)
        )
        .await;

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
#[bucket = "imaging"]
async fn invert(ctx: &Context, message: &Message, mut args: Args) -> CommandResult {
    let resolved = ImageResolver::new()
        .resolve(ctx, message, resolve_arg(&mut args))
        .await?;

    ImageExecutor::new(ctx, message)
        .function(invert_func)
        .run(resolved)
        .await
}

#[command]
#[bucket = "imaging"]
async fn huerotate(ctx: &Context, message: &Message, mut args: Args) -> CommandResult {
    let resolved = ImageResolver::new()
        .resolve(ctx, message, resolve_arg(&mut args))
        .await?;

    ImageExecutor::new(ctx, message)
        .function(huerotate_func)
        .run(resolved)
        .await
}

#[command]
#[bucket = "imaging"]
async fn caption(ctx: &Context, message: &Message, mut args: Args) -> CommandResult {
    let mut resolver = ImageResolver::new();
    let resolved = resolver
        .resolve(ctx, message, resolve_arg(&mut args))
        .await?;
    let arg = resolve_extra_arg(
        resolver.arg_resolved, &mut args
    );

    ImageExecutor::new(ctx, message)
        .function(caption_func)
        .arguments(vec![arg])
        .run(resolved)
        .await
}