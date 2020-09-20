use std::sync::Arc;

use actix_session::{CookieSession, Session};
use actix_web::{get, http::header, web, App, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;
use oauth2::basic::BasicClient;
use oauth2::http::{self, HeaderMap, Method};
use oauth2::reqwest::async_http_client;
use oauth2::url::Url;
use oauth2::{AccessToken, AsyncCodeTokenRequest, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, Scope, TokenResponse, TokenUrl};
use serde::{Deserialize, Serialize};
use serenity::model::id::UserId;
use tokio::sync::RwLock;

use super::web_socket::WebSocketSession;
use crate::config::{HttpConfig, OAuth2Config};
use crate::system::game::GameManager;

pub struct AppState {
    pub oauth2_client: BasicClient,
    pub auth_url: Url,
    pub game_manager: Arc<RwLock<GameManager>>,
}

pub async fn start_server(http_config: HttpConfig, oauth2_config: OAuth2Config, game_manager: Arc<RwLock<GameManager>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    HttpServer::new(move || {
        let client = BasicClient::new(
            ClientId::new(oauth2_config.client_id.clone()),
            Some(ClientSecret::new(oauth2_config.client_secret.clone())),
            AuthUrl::new(String::from("https://discord.com/api/oauth2/authorize")).unwrap(),
            Some(TokenUrl::new(String::from("https://discord.com/api/oauth2/token")).unwrap()),
        );

        let (auth_url, _) = client.authorize_url(CsrfToken::new_random).add_scope(Scope::new(String::from("identify"))).url();

        App::new()
            .data(AppState {
                oauth2_client: client,
                auth_url,
                game_manager: game_manager.clone(),
            })
            .wrap(CookieSession::private(&[0; 32]).secure(false))
            .service(login)
            .service(auth)
            .service(logout)
            .service(info)
            .service(web::resource("/socket").to(socket))
    })
    .bind(http_config.address.clone())?
    .run()
    .await
    .map_err(|e| e.into())
}

#[get("/login")]
async fn login(data: web::Data<AppState>) -> HttpResponse {
    HttpResponse::TemporaryRedirect().header(header::LOCATION, data.auth_url.to_string()).finish()
}

#[derive(Deserialize)]
struct AuthRequest {
    code: String,
    state: String,
}

#[get("/auth")]
async fn auth(session: Session, data: web::Data<AppState>, params: web::Query<AuthRequest>) -> HttpResponse {
    let code = AuthorizationCode::new(params.code.clone());
    let _state = CsrfToken::new(params.state.clone());

    let token = data.oauth2_client.exchange_code(code).request_async(async_http_client).await;
    let token = match &token {
        Ok(token) => token,
        Err(_) => return HttpResponse::Forbidden().body("Invalid token"),
    };

    let user_info = read_user(token.access_token()).await;

    session.set("user", user_info).unwrap();

    HttpResponse::TemporaryRedirect().header(header::LOCATION, "/info").finish()
}

#[derive(Serialize, Deserialize, Clone)]
pub struct UserInfo {
    pub id: UserId,
    pub username: String,
    pub discriminator: String,
}

async fn read_user(access_token: &AccessToken) -> UserInfo {
    let url = Url::parse("https://discord.com/api/users/@me").unwrap();

    let mut auth_header = String::from("Bearer ");
    auth_header.push_str(access_token.secret());

    let mut headers = HeaderMap::new();
    headers.insert(http::header::AUTHORIZATION, auth_header.parse().unwrap());

    let resp = async_http_client(oauth2::HttpRequest {
        url,
        method: Method::GET,
        headers,
        body: Vec::new(),
    })
    .await
    .expect("Request failed");

    serde_json::from_slice(&resp.body).unwrap()
}

#[get("/logout")]
async fn logout(session: Session) -> HttpResponse {
    session.remove("user");

    HttpResponse::NoContent().finish()
}

#[get("/info")]
async fn info(session: Session) -> HttpResponse {
    let user_info = match session.get::<UserInfo>("user").unwrap() {
        Some(info) => info,
        None => {
            return HttpResponse::TemporaryRedirect().header(header::LOCATION, "/login").finish();
        }
    };

    HttpResponse::Ok().json(user_info)
}

async fn socket(req: HttpRequest, stream: web::Payload, data: web::Data<AppState>, session: Session) -> Result<HttpResponse, actix_web::error::Error> {
    let user_info = match session.get::<UserInfo>("user").unwrap() {
        Some(user_info) => user_info,
        None => {
            return Ok(HttpResponse::Forbidden().finish());
        }
    };

    ws::start(WebSocketSession::new(user_info, data.game_manager.clone()), &req, stream)
}
