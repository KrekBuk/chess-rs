use serde::{Deserialize, Serialize};
use serde_json::Value;
use serenity::async_trait;
use tokio::sync::RwLockWriteGuard;

use crate::system::game::GameManager;

use crate::chess::board::{Color, Square};
use crate::chess::pieces::Type;
use crate::http::http_server::UserInfo;
use crate::system::game::Game;

use crate::chess::game::GameResult;
use crate::chess::moves::{Extra, NewMove};
use ProcessingError::*;

use std::str::FromStr;

#[derive(Serialize, Deserialize)]
pub struct State {
    pub user: UserInfo,
    pub game: Option<GameState>,
}

#[derive(Serialize, Deserialize)]
pub struct GameState {
    pub white: UserInfo,
    pub black: UserInfo,
    pub current_turn: Color,
    pub pieces: Vec<PieceInfo>,
    pub result: Option<GameResult>,
    pub winner: Option<Color>,
    pub highlighted_squares: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PieceInfo {
    pub piece_type: Type,
    pub color: Color,
    pub position: String,
    pub possible_valid_moves: Vec<String>,
    pub valid_moves: Vec<String>,
}

pub enum ProcessingError {
    NoOutput,
    InvalidProtocol,
}

fn make_game_state(current_player: &UserInfo, game: &Game) -> GameState {
    let turn = game.chess_game.state.current_turn;
    let our_turn = game.get_player_id_by_side(turn) == current_player.id;

    GameState {
        white: game.white_player.clone(),
        black: game.black_player.clone(),
        current_turn: turn,
        pieces: game
            .chess_game
            .state
            .board
            .state
            .pieces
            .iter()
            .map(|(_, piece)| {
                let show_moves = our_turn && piece.color == turn;

                PieceInfo {
                    piece_type: piece.piece_type,
                    color: piece.color,
                    position: piece.location.to_string(),
                    possible_valid_moves: if show_moves {
                        piece.valid_moves.iter().map(|square| square.to_string()).collect()
                    } else {
                        Vec::new()
                    },
                    valid_moves: if show_moves {
                        game.chess_game.state.board.get_valid_moves_for_piece(&piece).iter().map(|m| m.to.to_string()).collect()
                    } else {
                        Vec::new()
                    },
                }
            })
            .collect(),
        result: game.chess_game.result,
        winner: game.chess_game.result.and_then(|result| result.get_winner()),
        highlighted_squares: game.chess_game.state.board.highlighted_squares.iter().map(|square| square.to_string()).collect(),
    }
}
#[async_trait]
pub trait Handler {
    async fn fetch_user_info(&mut self) -> UserInfo;

    async fn get_game_manager(&mut self) -> RwLockWriteGuard<GameManager>;

    async fn handle(&mut self, text: String) -> Result<String, ProcessingError> {
        let value: Value = match serde_json::from_str(&text) {
            Ok(val) => val,
            Err(_) => {
                return Err(InvalidProtocol);
            }
        };

        let user = self.fetch_user_info().await;
        let mut game_manager = self.get_game_manager().await;
        let mut game = game_manager.get_game(user.id);

        let packet_type = value.get("type").and_then(|v| v.as_str());
        if let Some(packet_type) = packet_type {
            match packet_type {
                "get_state" => {}
                "make_move" => {
                    handle_make_move(&user, &value, &mut game)?;
                }
                _ => return Err(InvalidProtocol),
            };

            let state = State {
                user: user.clone(),
                game: game.map(|game| make_game_state(&user, game)),
            };

            Ok(serde_json::to_string_pretty(&state).unwrap())
        } else {
            Err(InvalidProtocol)
        }
    }
}

fn parse_square(value: Option<&Value>) -> Result<Square, ProcessingError> {
    value
        .and_then(|v| v.as_str())
        .ok_or(ProcessingError::InvalidProtocol)
        .and_then(|v| Square::from_str(v).map_err(|_| ProcessingError::InvalidProtocol))
}

fn handle_make_move(user: &UserInfo, value: &Value, game: &mut Option<&mut Game>) -> Result<(), ProcessingError> {
    match game {
        Some(game) => {
            if game.get_player_id_by_side(game.chess_game.state.current_turn) != user.id {
                return Ok(());
            }

            // TODO: Extra
            let from = parse_square(value.get("from"))?;
            let to = parse_square(value.get("to"))?;

            let _ = game.chess_game.make_move(NewMove { from, to, extra: Extra::None });

            Ok(())
        }
        None => Ok(()),
    }
}
