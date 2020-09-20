use std::time::{Duration, SystemTime};

use serenity::model::id::UserId;

use crate::chess::board::Color;
use crate::chess::game::Game as ChessGame;
use crate::http::http_server::UserInfo;

type PlayerId = UserId;

pub struct Game {
    pub white_player: UserInfo,
    pub black_player: UserInfo,
    pub chess_game: ChessGame,
}

impl Game {
    pub fn get_side_of_player(&self, player_id: PlayerId) -> Option<Color> {
        if self.white_player.id == player_id {
            Some(Color::White)
        } else if self.black_player.id == player_id {
            Some(Color::Black)
        } else {
            None
        }
    }

    pub fn get_player_id_by_side(&self, side: Color) -> PlayerId {
        match side {
            Color::White => self.white_player.id,
            Color::Black => self.black_player.id,
        }
    }
}

pub struct GameInvite {
    pub invitee: PlayerId,
    pub inviter: PlayerId,
    pub creation_time: SystemTime,
}

impl GameInvite {
    pub fn new(invitee: PlayerId, inviter: PlayerId) -> Self {
        Self {
            invitee,
            inviter,
            creation_time: SystemTime::now(),
        }
    }

    pub fn is_expired(&self) -> bool {
        self.creation_time.elapsed().unwrap() >= Duration::from_secs(30)
    }
}

pub struct GameManager {
    games: Vec<Game>,
    invites: Vec<GameInvite>,
}

impl GameManager {
    pub fn new() -> Self {
        Self::default()
    }

    fn remove_concluded_games(&mut self) {
        self.games.retain(|x| x.chess_game.result.is_none());
    }

    fn remove_expired_invites(&mut self) {
        self.invites.retain(|x| !x.is_expired());
    }

    pub fn create_game(&mut self, white_player: UserInfo, black_player: UserInfo) -> Option<&mut Game> {
        if self.get_game(white_player.id).is_some() || self.get_game(black_player.id).is_some() {
            return None;
        }

        let game = Game {
            white_player,
            black_player,
            chess_game: ChessGame::new(),
        };

        self.games.push(game);

        self.games.last_mut()
    }

    pub fn get_game(&mut self, player: PlayerId) -> Option<&mut Game> {
        self.remove_concluded_games();

        self.games.iter_mut().find(|game| game.white_player.id == player || game.black_player.id == player)
    }

    pub fn invite(&mut self, invitee: PlayerId, inviter: PlayerId) -> &GameInvite {
        self.remove_expired_invites();
        self.invites.push(GameInvite::new(invitee, inviter));
        self.invites.last().unwrap()
    }

    pub fn get_invite(&self, invitee: PlayerId, inviter: PlayerId) -> Option<&GameInvite> {
        self.invites.iter().find(|invite| invite.invitee == invitee && invite.inviter == inviter && !invite.is_expired())
    }

    pub fn remove_invite(&mut self, invitee: PlayerId, inviter: PlayerId) -> bool {
        self.remove_expired_invites();

        let len = self.invites.len();
        self.invites.retain(|invite| invite.invitee != invitee || invite.inviter != inviter);

        len != self.invites.len()
    }
}

impl Default for GameManager {
    fn default() -> Self {
        Self {
            games: Vec::new(),
            invites: Vec::new(),
        }
    }
}
