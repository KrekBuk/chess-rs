use serenity::client::Client;
use serenity::framework::standard::StandardFramework;
use serenity::prelude::{EventHandler, TypeMapKey};
use serenity::Error;

use crate::system::game::GameManager;
use crate::util::board_visualizer::BoardVisualizer;

use super::commands::game::GAME_GROUP;

struct Handler;

impl EventHandler for Handler {}

pub struct BotData {
    pub visualizer: BoardVisualizer,
    pub game_manager: GameManager,
}

impl TypeMapKey for BotData {
    type Value = BotData;
}

pub async fn start_bot(data: BotData) -> Result<(), Error> {
    // TODO: Token
    let mut client = Client::new("TOKEN")
        .type_map_insert::<BotData>(data)
        .event_handler(Handler)
        .framework(
            StandardFramework::new()
                .configure(|c| c.prefix("~"))
                .group(&GAME_GROUP),
        )
        .await
        .expect("client");

    client.start().await?;

    Ok(())
}
