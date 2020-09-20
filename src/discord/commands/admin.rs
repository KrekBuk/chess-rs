use serenity::framework::standard::{
    macros::{command, group},
    Args, CommandResult,
};
use serenity::model::channel::Message;
use serenity::model::id::UserId;
use serenity::model::misc::Mentionable;
use serenity::prelude::Context;

use super::GeneralError;
use crate::discord::bot::BotData;
use crate::discord::commands::game::send_board;
use crate::{chess::moves::NewMove, http::http_server::UserInfo};

#[derive(Error, Debug)]
pub enum AdminCommandError {
    #[error("Failed to resign.")]
    FailedToResign,
    #[error("Failed to make a draw.")]
    FailedToDraw,
    #[error("Failed to takeback a move.")]
    FailedToTakeback,
}

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
    let white = args.single::<UserId>()?.to_user(&ctx).await?;
    let black = args.single::<UserId>()?.to_user(&ctx).await?;

    let mut data = ctx.data.write().await;
    let data = data.get_mut::<BotData>().unwrap();
    let mut game_manager = data.game_manager.write().await;

    let game = game_manager.create_game(UserInfo::from(&white), UserInfo::from(&black)).ok_or(GeneralError::FailedToCreateGame)?;
    send_board(
        ctx,
        msg.channel_id,
        game,
        &data.visualizer.visualize(&game.chess_game.state.board).unwrap(),
        format!("{}, {}, the game has started!", white, black),
    )
    .await?;

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

    let game = game_manager.get_game(player).ok_or(GeneralError::PlayerNotInGame)?;

    game.chess_game.resign(game.get_side_of_player(player).unwrap()).map_err(|_| GeneralError::FailedToResign)?;

    send_board(
        ctx,
        msg.channel_id,
        game,
        &data.visualizer.visualize(&game.chess_game.state.board).unwrap(),
        String::from("The game was forcefully resigned. "),
    )
    .await?;

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

    let game = game_manager.get_game(player).ok_or(GeneralError::PlayerNotInGame)?;

    game.chess_game.draw().map_err(|_| AdminCommandError::FailedToDraw)?;

    send_board(
        ctx,
        msg.channel_id,
        game,
        &data.visualizer.visualize(&game.chess_game.state.board).unwrap(),
        String::from("The game was forcefully drawn. "),
    )
    .await?;

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

    let game = game_manager.get_game(player).ok_or(GeneralError::PlayerNotInGame)?;

    game.chess_game.takeback_move().map_err(|_| AdminCommandError::FailedToTakeback)?;
    send_board(
        ctx,
        msg.channel_id,
        game,
        &data.visualizer.visualize(&game.chess_game.state.board).unwrap(),
        format!("The move was taken back. Your turn {} ", game.get_player_id_by_side(game.chess_game.state.current_turn).mention()),
    )
    .await?;

    Ok(())
}

#[command]
#[description = "Make a move in a player's game"]
#[min_args(2)]
async fn force_move(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let player = args.single::<UserId>()?;
    let new_move = args.single::<NewMove>()?;

    let mut data = ctx.data.write().await;
    let data = data.get_mut::<BotData>().unwrap();
    let mut game_manager = data.game_manager.write().await;

    let game = game_manager.get_game(player).ok_or(GeneralError::PlayerNotInGame)?;

    game.chess_game.make_move(new_move).map_err(GeneralError::FailedToMove)?;
    send_board(
        ctx,
        msg.channel_id,
        game,
        &data.visualizer.visualize(&game.chess_game.state.board).unwrap(),
        format!("Your move {}", game.get_player_id_by_side(game.chess_game.state.current_turn).mention()),
    )
    .await?;

    Ok(())
}
