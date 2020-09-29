use crate::http::http_server::UserInfo;
use serenity::model::user::User;

impl From<&User> for UserInfo {
    fn from(user: &User) -> Self {
        UserInfo {
            id: user.id,
            discriminator: user.discriminator.to_string(),
            username: user.name.clone(),
            avatar: user.avatar.clone(),
        }
    }
}
