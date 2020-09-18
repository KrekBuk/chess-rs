use serenity::framework::standard::{
    macros::{command, group},
    Args, CommandResult,
};
use serenity::model::channel::Message;
use serenity::model::id::UserId;
use serenity::model::misc::Mentionable;
use serenity::prelude::Context;

use crate::chess::moves::NewMove;
use crate::discord::bot::BotData;
use crate::discord::commands::game::send_board;
use crate::http::http_server::UserInfo;

#[group]
#[prefixes("admin")]
#[description = "Admin commands."]
#[commands(start, force_resign, force_draw, force_takeback, force_move)]
#[owners_only]
pub struct Admin;

#[command]
#[description = "Start a game"]
#[min_args(2)]
async fn start(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let white = args.single::<UserId>()?.to_user(&ctx.http).await?;
    let black = args.single::<UserId>()?.to_user(&ctx.http).await?;

    let mut data = ctx.data.write().await;
    let data = data.get_mut::<BotData>().unwrap();
    let mut game_manager = data.game_manager.write().await;

    match game_manager.create_game(UserInfo::from(&white), UserInfo::from(&black)) {
        Some(game) => {
            send_board(
                ctx,
                msg.channel_id,
                game,
                &data.visualizer.visualize(&game.chess_game.state.board).unwrap(),
                format!("{}, {}, the game has started!", white, black),
            )
            .await?;
        }
        None => {
            msg.reply(&ctx.http, "Failed to create a game, maybe you're already in one?").await?;
        }
    };

    Ok(())
}

#[command]
#[description = "Forcefully resign as a player. "]
#[min_args(1)]
async fn force_resign(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let player = args.single::<UserId>()?;

    let mut data = ctx.data.write().await;
    let data = data.get_mut::<BotData>().unwrap();
    let mut game_manager = data.game_manager.write().await;

    let game = match game_manager.get_game(player) {
        Some(game) => game,
        None => {
            msg.reply(&ctx.http, "This player is not in a game.").await?;

            return Ok(());
        }
    };

    match game.chess_game.resign(game.get_side_of_player(player).unwrap()) {
        Ok(_) => {
            send_board(
                ctx,
                msg.channel_id,
                game,
                &data.visualizer.visualize(&game.chess_game.state.board).unwrap(),
                String::from("The game was forcefully resigned. "),
            )
            .await?;
        }
        Err(_) => {
            msg.reply(&ctx.http, "Failed to resign. ").await?;
        }
    }

    Ok(())
}

#[command]
#[description = "Forcefully draw a player's game. "]
#[min_args(1)]
async fn force_draw(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let player = args.single::<UserId>()?;

    let mut data = ctx.data.write().await;
    let data = data.get_mut::<BotData>().unwrap();
    let mut game_manager = data.game_manager.write().await;

    let game = match game_manager.get_game(player) {
        Some(game) => game,
        None => {
            msg.reply(&ctx.http, "This player is not in a game.").await?;

            return Ok(());
        }
    };

    match game.chess_game.draw() {
        Ok(_) => {
            send_board(
                ctx,
                msg.channel_id,
                game,
                &data.visualizer.visualize(&game.chess_game.state.board).unwrap(),
                String::from("The game was forcefully drawn. "),
            )
            .await?;
        }
        Err(_) => {
            msg.reply(&ctx.http, "Failed to make a draw. ").await?;
        }
    }

    Ok(())
}

#[command]
#[description = "Forcefully resign as a player. "]
#[min_args(1)]
async fn force_takeback(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let player = args.single::<UserId>()?;

    let mut data = ctx.data.write().await;
    let data = data.get_mut::<BotData>().unwrap();
    let mut game_manager = data.game_manager.write().await;

    let game = match game_manager.get_game(player) {
        Some(game) => game,
        None => {
            msg.reply(&ctx.http, "This player is not in a game.").await?;

            return Ok(());
        }
    };

    match game.chess_game.takeback_move() {
        Ok(_) => {
            send_board(
                ctx,
                msg.channel_id,
                game,
                &data.visualizer.visualize(&game.chess_game.state.board).unwrap(),
                format!("The move was taken back. Your turn {} ", game.get_player_id_by_side(game.chess_game.state.current_turn).mention()),
            )
            .await?;
        }
        Err(_) => {
            msg.reply(&ctx.http, "Failed to takeback a move. ").await?;
        }
    }

    Ok(())
}

#[command]
#[description = "Make a move in a player's game"]
#[min_args(2)]
async fn force_move(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let player = args.single::<UserId>()?;
    let move_ = args.single::<NewMove>()?;

    let mut data = ctx.data.write().await;
    let data = data.get_mut::<BotData>().unwrap();
    let mut game_manager = data.game_manager.write().await;

    let game = match game_manager.get_game(player) {
        Some(game) => game,
        None => {
            msg.reply(&ctx.http, "This player is not in a game.").await?;

            return Ok(());
        }
    };

    match game.chess_game.make_move(move_) {
        Ok(_) => {
            send_board(
                ctx,
                msg.channel_id,
                game,
                &data.visualizer.visualize(&game.chess_game.state.board).unwrap(),
                format!("Your move {}", game.get_player_id_by_side(game.chess_game.state.current_turn).mention()),
            )
            .await?;
        }
        Err(e) => {
            msg.reply(&ctx.http, format!("Invalid move: {:?}", e)).await?;
        }
    }

    Ok(())
}
