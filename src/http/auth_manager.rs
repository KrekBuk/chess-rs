use crate::http::http_server::UserInfo;
use serenity::model::id::UserId;

use std::collections::HashMap;

use rand::distributions::Alphanumeric;
use rand::Rng;

pub struct AuthenticationManager {
    map: HashMap<String, UserInfo>,
}

pub enum AuthenticationError {
    InvalidToken,
}

impl AuthenticationManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn generate_new(&mut self, user: UserInfo) -> String {
        self.invalidate_for(user.id);

        let token: String = std::iter::repeat(()).map(|()| rand::thread_rng().sample(Alphanumeric)).take(32).collect();
        self.map.insert(token.clone(), user);
        token
    }

    pub fn invalidate_for(&mut self, id: UserId) {
        self.map.retain(|_, v| v.id != id);
    }

    pub fn get_for_token(&self, token: String) -> Result<&UserInfo, AuthenticationError> {
        self.map.get(&token).ok_or(AuthenticationError::InvalidToken)
    }

    pub fn get_token_for_user(&self, user: &UserInfo) -> Option<String> {
        self.map.iter().find(|(_, value)| value.id == user.id).map(|(key, _)| key.clone())
    }

    pub fn get_or_generate_token_for_user(&mut self, user: UserInfo) -> String {
        match self.get_token_for_user(&user) {
            Some(token) => token,
            None => self.generate_new(user),
        }
    }
}

impl Default for AuthenticationManager {
    fn default() -> Self {
        Self { map: HashMap::new() }
    }
}
