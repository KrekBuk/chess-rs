use std::collections::HashSet;
use std::sync::Arc;

use once_cell::sync::Lazy;
use regex::Regex;
use serenity::async_trait;
use serenity::framework::standard::{
    help_commands,
    macros::{help, hook},
    Args, CommandGroup, CommandResult, Delimiter, DispatchError, HelpOptions, StandardFramework,
};
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::id::{ChannelId, UserId};
use serenity::prelude::{Context, EventHandler, TypeMapKey};
use serenity::{client::Client, framework::standard::CommandError};
use tokio::sync::RwLock;

use super::commands::admin::ADMIN_GROUP;
use super::commands::game::make_move;
use super::commands::game::GAMECOMMANDS_GROUP;
use crate::config::DiscordConfig;
use crate::system::game::GameManager;
use crate::util::board_visualizer::BoardVisualizer;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

pub struct BotData {
    pub visualizer: BoardVisualizer,
    pub game_manager: Arc<RwLock<GameManager>>,
    pub prefix: String,
}

impl TypeMapKey for BotData {
    type Value = BotData;
}

pub async fn start_bot(config: DiscordConfig, data: BotData) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut client = Client::new(config.token.clone())
        .type_map_insert::<BotData>(data)
        .event_handler(Handler)
        .framework(
            StandardFramework::new()
                .configure(|c| {
                    c.prefix(&config.prefix.clone())
                        .allowed_channels(config.allowed_channels.iter().map(|id| ChannelId(*id)).collect())
                        .owners(config.owners.iter().map(|id| UserId(*id)).collect())
                })
                .on_dispatch_error(dispatch_error)
                .unrecognised_command(unknown_command)
                .normal_message(normal_message)
                .after(command_error_handler)
                .help(&MY_HELP)
                .group(&ADMIN_GROUP)
                .group(&GAMECOMMANDS_GROUP),
        )
        .await
        .expect("client");

    client.start().await?;

    Ok(())
}

#[hook]
async fn dispatch_error(ctx: &Context, msg: &Message, error: DispatchError) {
    match error {
        DispatchError::NotEnoughArguments { min, given: _ } => {
            let _ = msg.reply(&ctx, &format!("Not enough arguments. {} required.", min)).await;
        }
        DispatchError::OnlyForOwners => {
            let _ = msg.reply(&ctx, "Only for owners.").await;
        }
        _ => {
            let _ = msg.reply(&ctx, format!("Error processing command: {:?}", error)).await;
        }
    }
}

#[hook]
async fn unknown_command(ctx: &Context, msg: &Message, unknown_command_name: &str) {
    let _ = msg.reply(&ctx, format!("Could not find command named '{}'", unknown_command_name)).await;
}

#[help]
async fn my_help(context: &Context, msg: &Message, args: Args, help_options: &'static HelpOptions, groups: &[&'static CommandGroup], owners: HashSet<UserId>) -> CommandResult {
    let _ = help_commands::with_embeds(context, msg, args, help_options, groups, owners).await;
    Ok(())
}

#[hook]
async fn normal_message(ctx: &Context, msg: &Message) {
    static REGEX: Lazy<Regex> = Lazy::new(|| Regex::new("^([A-H][1-8]){2}$").unwrap());

    let args;
    {
        let data = ctx.data.read().await;
        let data = data.get::<BotData>().unwrap();

        if !msg.content.starts_with(&data.prefix) {
            return;
        }

        let mut move_str = msg.content.to_uppercase();
        move_str.drain(0..data.prefix.len());

        if !REGEX.is_match(&move_str) {
            return;
        }

        args = Args::new(&move_str, &[Delimiter::Single(' ')])
    }

    let _ = make_move(ctx, msg, args).await;
}

#[hook]
async fn command_error_handler(ctx: &Context, msg: &Message, _: &str, error: Result<(), CommandError>) {
    if let Err(why) = error {
        msg.reply(ctx, why).await.unwrap();
    }
}
