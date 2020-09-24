use serde::{Deserialize, Serialize};
use serde_json::Value;
use serenity::async_trait;
use tokio::sync::RwLockWriteGuard;

use crate::chess::board::{Color, Square};
use crate::chess::pieces::Type;
use crate::http::http_server::UserInfo;
use crate::system::game::{Game, GameManager};

use crate::chess::game::{Game as ChessGame, GameResult};
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
    pub draw_offers: Vec<String>,
    pub takeback_offers: Vec<String>,
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
    OldState,
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
        draw_offers: map_colors_to_ids(game, &game.chess_game.state.draw_offers),
        takeback_offers: map_colors_to_ids(game, &game.chess_game.state.takeback_offers),
    }
}

#[async_trait]
pub trait Handler {
    async fn fetch_user_info(&mut self) -> UserInfo;

    async fn get_game_manager(&mut self) -> RwLockWriteGuard<GameManager>;

    async fn handle(&mut self, text: String) -> Result<Option<String>, ProcessingError> {
        let value: Value = match serde_json::from_str(&text) {
            Ok(val) => val,
            Err(_) => {
                return Err(InvalidProtocol);
            }
        };

        let user = self.fetch_user_info().await;
        let mut game_manager = self.get_game_manager().await;
        let game = game_manager.get_game(user.id);

        let packet_type = value.get("type").and_then(|v| v.as_str());
        if let Some(packet_type) = packet_type {
            match packet_type {
                "get_state" => return Ok(Some(make_state(&user, &game))),
                "make_move" => {
                    handle_make_move(&user, &value, game)?;
                }
                "offer_draw" => {
                    handle_simple_function(&user, game, ChessGame::offer_draw)?;
                }
                "offer_takeback" => {
                    handle_simple_function(&user, game, ChessGame::offer_takeback)?;
                }
                "resign" => {
                    handle_simple_function(&user, game, ChessGame::resign)?;
                }
                _ => return Err(InvalidProtocol),
            };

            Ok(None)
        } else {
            Err(InvalidProtocol)
        }
    }
}

pub fn make_state(user: &UserInfo, game: &Option<&mut Game>) -> String {
    let state = State {
        user: user.clone(),
        game: game.as_ref().map(|game| make_game_state(&user, game)),
    };

    serde_json::to_string_pretty(&state).unwrap()
}

fn parse_square(value: Option<&Value>) -> Result<Square, ProcessingError> {
    value
        .and_then(|v| v.as_str())
        .ok_or(ProcessingError::InvalidProtocol)
        .and_then(|v| Square::from_str(v).map_err(|_| ProcessingError::InvalidProtocol))
}

fn map_colors_to_ids(game: &Game, colors: &Vec<Color>) -> Vec<String> {
    colors.iter().map(|color| game.get_player_id_by_side(*color).to_string()).collect()
}

fn handle_make_move(user: &UserInfo, value: &Value, game: Option<&mut Game>) -> Result<(), ProcessingError> {
    let game = game.ok_or(OldState)?;
    if game.get_player_id_by_side(game.chess_game.state.current_turn) != user.id {
        return Err(OldState);
    }

    // TODO: Extra
    let from = parse_square(value.get("from"))?;
    let to = parse_square(value.get("to"))?;

    let _ = game.chess_game.make_move(NewMove {
        from,
        to,
        extra: Extra::Promotion(Type::Queen),
    });

    Ok(())
}

fn handle_simple_function<'a, F, R>(user: &UserInfo, game: Option<&'a mut Game>, function: F) -> Result<(), ProcessingError>
where
    F: FnOnce(&'a mut ChessGame, Color) -> R,
{
    let game = game.ok_or(OldState)?;
    let color = game.get_side_of_player(user.id).ok_or(OldState)?;
    function(&mut game.chess_game, color);

    Ok(())
}
