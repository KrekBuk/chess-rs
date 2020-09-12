use serenity::http::AttachmentType;
use serenity::model::channel::Message;
use serenity::prelude::{Context};

use crate::chess::board::Square;
use crate::chess::moves::Extra;
use crate::discord::bot::BotData;
use serenity::framework::standard::{
    macros::{command, group},
    Args, CommandResult,
};

#[group]
#[prefixes("game")]
#[commands(make_move)]
pub struct Game;

#[command]
#[aliases("move")]
async fn make_move(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let m = args.single::<String>()?.to_uppercase();

    if m.len() != 4 {
        msg.reply(&ctx.http, "Invalid move").await?;
        return Ok(());
    }

    let move_from = Square::from_string(&*m[0..2].to_uppercase());
    let move_to = Square::from_string(&*m[2..4].to_uppercase());

    if move_from.is_none() || move_to.is_none() {
        msg.reply(&ctx.http, "Invalid move").await?;
        return Ok(());
    }

    let mut data = ctx.data.write().await;
    let data = data.get_mut::<BotData>().unwrap();

    let game = match data.game_manager.get_game(msg.author.id) {
        Some(game) => game,
        None => {
            msg.reply(&ctx.http, "You are not in a game").await?;
            return Ok(());
        }
    };

    match game
        .chess_game
        .make_move(move_from.unwrap(), move_to.unwrap(), Extra::None)
    {
        Ok(_) => {
            msg.channel_id
                .send_files(
                    &ctx.http,
                    std::iter::once(AttachmentType::from((
                        data.visualizer
                            .visualize(&game.chess_game.state.board)
                            .unwrap()
                            .as_slice(),
                        "board.png",
                    ))),
                    |f| {
                        if let Some(result) = game.chess_game.result {
                            f.content(format!(
                                "Game concluded. \nResult: {:?}. \nWinner {:?}",
                                result,
                                result.get_winner()
                            ));
                        }

                        f
                    },
                )
                .await?;
        }
        Err(e) => {
            msg.reply(&ctx.http, format!("Invalid move: {:?}", e))
                .await?;
        }
    }

    Ok(())
}
