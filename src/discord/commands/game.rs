use serenity::framework::standard::{
    macros::{command, group},
    Args, CommandResult,
};
use serenity::http::AttachmentType;
use serenity::model::channel::Message;
use serenity::model::id::{ChannelId, UserId};
use serenity::model::misc::Mentionable;
use serenity::prelude::Context;
use serenity::Result;

use super::GeneralError;
use crate::chess::game::GameResult;
use crate::chess::moves::NewMove;
use crate::discord::bot::BotData;
use crate::http::http_server::UserInfo;
use crate::system::game::Game;

#[derive(Error, Debug)]
enum CommandError {
    #[error("You cannot invite yourself")]
    CannotInviteSelf,
    #[error("Invalid user.")]
    InvalidUser,
    #[error("You are not in a game.")]
    NotInGame,
    #[error("You are already in a game.")]
    AlreadyInGame,
    #[error("This user is already in a game.")]
    UserAlreadyInGame,
    #[error("You already invited this user!")]
    AlreadyInvited,
    #[error("There are no invites from this user.")]
    NoInvitation,
    #[error("Failed to send a takeback request.")]
    FailedToTakeback,
    #[error("Failed to send a draw request.")]
    FailedToDraw,
}

#[group]
#[prefixes("game")]
#[description = "Game-related commands."]
#[commands(invite, accept, decline, draw, resign, make_move, board, takeback)]
#[only_in(guilds)]
pub struct GameCommands;

#[command]
#[description = "Invite someone to a game."]
#[min_args(1)]
async fn invite(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let mention = args.single::<UserId>()?;

    if mention == msg.author.id {
        return Err(CommandError::CannotInviteSelf.into());
    }

    let user = mention.to_user(&ctx).await.map_err(|_| CommandError::InvalidUser)?;

    let mut data = ctx.data.write().await;
    let data = data.get_mut::<BotData>().unwrap();
    let mut game_manager = data.game_manager.write().await;

    if game_manager.get_game(msg.author.id).is_some() {
        return Err(CommandError::AlreadyInGame.into());
    }

    if game_manager.get_game(user.id).is_some() {
        return Err(CommandError::UserAlreadyInGame.into());
    }

    if game_manager.get_invite(user.id, msg.author.id).is_some() {
        return Err(CommandError::AlreadyInvited.into());
    }

    game_manager.invite(user.id, msg.author.id);

    msg.channel_id
        .say(
            &ctx,
            format!(
                "Hey, {mentionedUser} you were invited to a game of chess.\nType {prefix}game accept {author} to accept.\nType {prefix}game decline {author} to decline",
                prefix = data.prefix,
                mentionedUser = user,
                author = msg.author
            ),
        )
        .await?;

    Ok(())
}

#[command]
#[description = "Accept a game invitation."]
#[min_args(1)]
async fn accept(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let mention = args.single::<UserId>()?;
    let other_user = mention.to_user(&ctx).await?;

    let mut data = ctx.data.write().await;
    let data = data.get_mut::<BotData>().unwrap();
    let mut game_manager = data.game_manager.write().await;

    if game_manager.get_invite(msg.author.id, mention).is_none() {
        return Err(CommandError::NoInvitation.into());
    }
    game_manager.remove_invite(msg.author.id, mention);

    let game = game_manager
        .create_game(UserInfo::from(&other_user), UserInfo::from(&msg.author))
        .ok_or(GeneralError::FailedToCreateGame)?;

    send_board(
        ctx,
        msg.channel_id,
        game,
        &data.visualizer.visualize(&game.chess_game.state.board).unwrap(),
        format!("{}, {}, the game has started!", msg.author.id.mention(), mention.mention()),
    )
    .await?;

    Ok(())
}

#[command]
#[description = "Decline a game invitation."]
#[min_args(1)]
async fn decline(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let mention = args.single::<UserId>()?;

    let mut data = ctx.data.write().await;
    let data = data.get_mut::<BotData>().unwrap();

    if !data.game_manager.write().await.remove_invite(msg.author.id, mention) {
        return Err(CommandError::NoInvitation.into());
    }

    msg.channel_id.say(&ctx, format!("{}, your invitation was declined", mention.mention())).await?;

    Ok(())
}

#[command]
#[description = "Send a draw request."]
async fn draw(ctx: &Context, msg: &Message) -> CommandResult {
    let mut data = ctx.data.write().await;
    let data = data.get_mut::<BotData>().unwrap();
    let mut game_manager = data.game_manager.write().await;

    let game = game_manager.get_game(msg.author.id).ok_or(CommandError::NotInGame)?;

    let author_color = game.get_side_of_player(msg.author.id).unwrap();
    let other_player = game.get_player_id_by_side(author_color.get_opposite());

    let result = game.chess_game.offer_draw(author_color).map_err(|_| CommandError::FailedToDraw)?;

    match result {
        GameResult::DrawAgreed => {
            send_board(
                ctx,
                msg.channel_id,
                game,
                &data.visualizer.visualize(&game.chess_game.state.board).unwrap(),
                format!("{} and {} agreed to a draw.", msg.author.id.mention(), other_player.mention()),
            )
            .await?;
        }
        _ => {
            msg.channel_id
                .say(
                    &ctx,
                    format!("{}, {} wants a draw. Type {}game draw to accept", other_player.mention(), msg.author.id.mention(), data.prefix),
                )
                .await?;
        }
    }

    Ok(())
}

#[command]
#[description = "Resign the game."]
async fn resign(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let mut data = ctx.data.write().await;
    let data = data.get_mut::<BotData>().unwrap();
    let mut game_manager = data.game_manager.write().await;

    let game = game_manager.get_game(msg.author.id).ok_or(CommandError::NotInGame)?;

    let author_color = game.get_side_of_player(msg.author.id).unwrap();

    game.chess_game.resign(author_color).map_err(|_| GeneralError::FailedToResign)?;
    send_board(
        ctx,
        msg.channel_id,
        game,
        &data.visualizer.visualize(&game.chess_game.state.board).unwrap(),
        format!("{} resigned. ", msg.author.id.mention()),
    )
    .await?;

    Ok(())
}

#[command]
#[aliases("move")]
#[description = "Make a move on the board."]
#[min_args(1)]
pub async fn make_move(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let m = match args.single::<NewMove>() {
        Ok(m) => m,
        Err(_) => {
            msg.reply(&ctx, "Invalid move").await?;
            return Ok(());
        }
    };

    let mut data = ctx.data.write().await;
    let data = data.get_mut::<BotData>().unwrap();
    let mut game_manager = data.game_manager.write().await;

    let game = game_manager.get_game(msg.author.id).ok_or(CommandError::NotInGame)?;

    if game.get_player_id_by_side(game.chess_game.state.current_turn) != msg.author.id {
        msg.reply(&ctx, "Not your move.").await?;
        return Ok(());
    }

    game.chess_game.make_move(m).map_err(GeneralError::FailedToMove)?;
    send_board(
        &ctx,
        msg.channel_id,
        game,
        &data.visualizer.visualize(&game.chess_game.state.board).unwrap(),
        format!("Your move {}", game.get_player_id_by_side(game.chess_game.state.current_turn).mention()),
    )
    .await?;

    Ok(())
}

#[command]
#[description = "Re-send the current board."]
async fn board(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let mut data = ctx.data.write().await;
    let data = data.get_mut::<BotData>().unwrap();
    let mut game_manager = data.game_manager.write().await;

    let user = if args.is_empty() { msg.author.id } else { args.single::<UserId>()? };

    let game = match game_manager.get_game(user) {
        Some(game) => game,
        None => {
            if user == msg.author.id {
                msg.reply(&ctx, "You are not in a game.").await?;
            } else {
                msg.reply(&ctx, "This player is not in a game.").await?;
            }

            return Ok(());
        }
    };

    send_board(ctx, msg.channel_id, game, &data.visualizer.visualize(&game.chess_game.state.board).unwrap(), String::from("")).await?;

    Ok(())
}

#[command]
#[description = "Send a takeback request"]
async fn takeback(ctx: &Context, msg: &Message) -> CommandResult {
    let mut data = ctx.data.write().await;
    let data = data.get_mut::<BotData>().unwrap();
    let mut game_manager = data.game_manager.write().await;

    let game = game_manager.get_game(msg.author.id).ok_or(CommandError::NotInGame)?;

    let author_color = game.get_side_of_player(msg.author.id).unwrap();
    let other_player = game.get_player_id_by_side(author_color.get_opposite());

    let result = game.chess_game.offer_takeback(author_color).map_err(|_| CommandError::FailedToTakeback)?;

    if result {
        send_board(
            ctx,
            msg.channel_id,
            game,
            &data.visualizer.visualize(&game.chess_game.state.board).unwrap(),
            format!("Takeback accepted. Your move {}.", game.get_player_id_by_side(game.chess_game.state.current_turn).mention()),
        )
        .await?;
    } else {
        msg.channel_id
            .say(
                &ctx,
                format!("{}, {} wants a takeback. Type {}game takeback to accept", other_player.mention(), msg.author.id.mention(), data.prefix),
            )
            .await?;
    }

    Ok(())
}

pub async fn send_board(ctx: &Context, channel: ChannelId, game: &Game, vec: &[u8], header: String) -> Result<Message> {
    channel
        .send_files(&ctx, std::iter::once(AttachmentType::from((vec, "board.png"))), |f| {
            let mut content = String::new();
            content.push_str(&header);

            if let Some(result) = game.chess_game.result {
                content.push_str("The game has concluded.\n");
                content.push_str(&result.pretty_message());
                content.push('\n');

                if let Some(winner) = result.get_winner() {
                    content.push_str("Winner: ");
                    content.push_str(&game.get_player_id_by_side(winner).mention());
                    content.push_str(". Loser: ");
                    content.push_str(&game.get_player_id_by_side(winner.get_opposite()).mention());
                } else {
                    content.push_str("The game was drawn. ");
                }
            }

            f.content(content);
            f
        })
        .await
}
